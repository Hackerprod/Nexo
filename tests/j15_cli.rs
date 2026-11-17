//! Public CLI evidence for the executable slice in F0-0003 §11 and A2 §12.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const PROFILE: &str = "f0-0003-b-v1";
const EARLIER: &str = "f0-0002-b-v2";
const EX04: &str = "gen fn Cuadrados(limite:view I64,r:move MockResource)->I64 {\n    var n=0;\n    var cuadrado=0;\n    var impar=1;\n    while n<limite {\n        Inspect(r);\n        yield cuadrado;\n        n=n+1;\n        if n<limite {\n            cuadrado=cuadrado+impar;\n            impar=impar+2;\n        }\n    }\n}\nfn Main()->Result<(),I64> {\n    let limite=MockRequest()?;\n    if limite<0 {return Err(41);}\n    if 1000<limite {return Err(41);}\n    for valor in Cuadrados(limite,MockOpen(4)) {\n        MockWriteI64(valor)?;\n    }\n    return Ok(());\n}";
const EX04_R: &str = "gen fn Cuadrados(limite:view I64,r:move MockResource)->I64 {\n    var n=0;\n    var cuadrado=0;\n    var impar=1;\n    while n<limite {\n        Inspect(r);\n        yield cuadrado;\n        n=n+1;\n        if n<limite {\n            cuadrado=9223372036854775807+1;\n            impar=impar+2;\n        }\n    }\n}\nfn Main()->Result<(),I64> {\n    let limite=MockRequest()?;\n    if limite<0 {return Err(41);}\n    if 1000<limite {return Err(41);}\n    for valor in Cuadrados(limite,MockOpen(4)) {\n        MockWriteI64(valor)?;\n    }\n    return Ok(());\n}";
const FLOW1: &str = "fn Sumar(records: view Array<Result<I64,I64>>) -> Result<I64,I64> with no_alloc,no_block,no_suspend,no_io {\n var i=0;\n var total=0;\n while i<Len(records){let value=records[i]?;total=TryAdd(total,value)?;i=i+1;}\n return Ok(total);\n}\nfn Main() -> Result<(),I64> {\n let file=MockOpen(7);\n let records=MockReadRecords(file)?;\n let total=Sumar(records)?;\n MockWriteI64(total)?;\n return Ok(());\n}";
const FLOW2: &str = "fn Incrementar(xs: edit Array<I64>) with no_alloc,no_block,no_suspend,no_io {var i=0;while i<Len(xs){xs[i]=xs[i]+1;i=i+1;}} fn Main(){var original=[1,2];var copia=original;Incrementar(edit copia);AssertEq(original,[1,2]);AssertEq(copia,[2,3]);}";
const FLOW3: &str = "fn Responder(route: view I64)->I64 with no_alloc,no_block,no_suspend,no_io{return if route==0 then {value:200;} else {value:404;};} fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(Responder(route))?;return Ok(());}";

// A deliberately small JSON reader for the CLI report. It is local to this
// integration test and has no dependency on Nexo's parser or JSON transport.
#[derive(Clone, Debug, PartialEq, Eq)]
enum J {
    Null,
    Bool(bool),
    Num(i64),
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}
impl J {
    fn at(&self, key: &str) -> &J {
        let J::Obj(fields) = self else {
            panic!("not an object: {self:?}")
        };
        fields
            .get(key)
            .unwrap_or_else(|| panic!("missing {key}: {self:?}"))
    }
    fn array(&self) -> &[J] {
        let J::Arr(items) = self else {
            panic!("not an array: {self:?}")
        };
        items
    }
}
struct Reader<'a> {
    text: &'a str,
    pos: usize,
}
impl Reader<'_> {
    fn ws(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn take(&mut self, byte: u8) {
        self.ws();
        assert_eq!(
            self.text.as_bytes().get(self.pos),
            Some(&byte),
            "{}",
            &self.text[self.pos..]
        );
        self.pos += 1;
    }
    fn value(&mut self) -> J {
        self.ws();
        match self.text.as_bytes()[self.pos] {
            b'{' => {
                self.pos += 1;
                let mut fields = BTreeMap::new();
                self.ws();
                if self.text.as_bytes()[self.pos] != b'}' {
                    loop {
                        let key = self.quoted();
                        self.take(b':');
                        assert!(fields.insert(key, self.value()).is_none());
                        self.ws();
                        if self.text.as_bytes()[self.pos] != b',' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                self.take(b'}');
                J::Obj(fields)
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                self.ws();
                if self.text.as_bytes()[self.pos] != b']' {
                    loop {
                        items.push(self.value());
                        self.ws();
                        if self.text.as_bytes()[self.pos] != b',' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                self.take(b']');
                J::Arr(items)
            }
            b'"' => J::Str(self.quoted()),
            b't' => {
                self.literal("true");
                J::Bool(true)
            }
            b'f' => {
                self.literal("false");
                J::Bool(false)
            }
            b'n' => {
                self.literal("null");
                J::Null
            }
            b'-' | b'0'..=b'9' => {
                let begin = self.pos;
                while self
                    .text
                    .as_bytes()
                    .get(self.pos)
                    .is_some_and(|b| b.is_ascii_digit() || *b == b'-')
                {
                    self.pos += 1;
                }
                J::Num(self.text[begin..self.pos].parse().unwrap())
            }
            _ => panic!("invalid JSON at {}", self.pos),
        }
    }
    fn literal(&mut self, word: &str) {
        assert!(self.text[self.pos..].starts_with(word));
        self.pos += word.len();
    }
    fn quoted(&mut self) -> String {
        self.take(b'"');
        let mut result = String::new();
        loop {
            let ch = self.text[self.pos..]
                .chars()
                .next()
                .expect("unterminated string");
            self.pos += ch.len_utf8();
            match ch {
                '"' => return result,
                '\\' => {
                    let escape = self.text.as_bytes()[self.pos];
                    self.pos += 1;
                    result.push(match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let code = u32::from_str_radix(&self.text[self.pos..self.pos + 4], 16)
                                .unwrap();
                            self.pos += 4;
                            char::from_u32(code).unwrap()
                        }
                        _ => panic!("invalid escape"),
                    });
                }
                _ => result.push(ch),
            }
        }
    }
}
fn parse(text: &str) -> J {
    let mut reader = Reader { text, pos: 0 };
    let value = reader.value();
    reader.ws();
    assert_eq!(reader.pos, text.len(), "extra JSON");
    value
}

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    dir: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "nexo-j15-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let file = dir.join("source.n");
        fs::write(&file, source).unwrap();
        Self { dir, source: file }
    }
    fn env(&self, json: &str) -> PathBuf {
        let file = self.dir.join("env.json");
        fs::write(&file, json).unwrap();
        file
    }
    fn invoke(&self, command: &str, profile: &str, format: &str, env: Option<&Path>) -> Output {
        let mut call = Command::new(env!("CARGO_BIN_EXE_nexo"));
        call.arg(command)
            .arg(&self.source)
            .args(["--profile", profile, "--format", format]);
        if let Some(path) = env {
            call.arg("--mock-env").arg(path);
        }
        call.output().unwrap()
    }
    fn report(&self, command: &str, profile: &str, env: Option<&Path>) -> (i32, J) {
        let output = self.invoke(command, profile, "json", env);
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert_eq!(text.lines().count(), 1, "{text}");
        (output.status.code().unwrap(), parse(text.trim_end()))
    }
    fn check(&self, profile: &str) {
        let (code, doc) = self.report("check", profile, None);
        assert_eq!(code, 0);
        assert_eq!(doc.at("outcome").at("kind"), &J::Str("checked".into()));
        empty_report(&doc);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[derive(Clone)]
struct Expect {
    exit: i32,
    kind: &'static str,
    error: Option<i64>,
    observations: Vec<(&'static str, String)>,
}
impl Expect {
    fn new(exit: i32, kind: &'static str, error: Option<i64>) -> Self {
        Self {
            exit,
            kind,
            error,
            observations: Vec::new(),
        }
    }
    fn event(&mut self, event: impl Into<String>) {
        self.observations.push(("event", event.into()));
    }
    fn stdout(&mut self, text: impl Into<String>) {
        self.observations.push(("stdout", text.into()));
    }
}
fn empty_report(doc: &J) {
    assert_eq!(doc.at("stdout"), &J::Arr(vec![]));
    assert_eq!(doc.at("events"), &J::Arr(vec![]));
    assert_eq!(doc.at("observations"), &J::Arr(vec![]));
    assert_eq!(doc.at("observation_bytes"), &J::Num(0));
    assert_eq!(doc.at("reserved_observation_bytes"), &J::Num(0));
    assert_eq!(doc.at("diagnostics"), &J::Arr(vec![]));
}
fn assert_report(actual: (i32, J), profile: &str, expected: &Expect) {
    let (exit, doc) = actual;
    assert_eq!(exit, expected.exit, "{doc:?}");
    assert_eq!(doc.at("schema_version"), &J::Num(1));
    assert_eq!(doc.at("profile_id"), &J::Str(profile.into()));
    let mut outcome = BTreeMap::from([("kind".into(), J::Str(expected.kind.into()))]);
    if let Some(error) = expected.error {
        outcome.insert("error".into(), J::Str(error.to_string()));
    }
    assert_eq!(doc.at("outcome"), &J::Obj(outcome));
    let observations = expected
        .observations
        .iter()
        .map(|(channel, text)| {
            J::Obj(BTreeMap::from([
                ("channel".into(), J::Str((*channel).into())),
                ("text".into(), J::Str(text.clone())),
            ]))
        })
        .collect();
    assert_eq!(doc.at("observations"), &J::Arr(observations));
    let events = expected
        .observations
        .iter()
        .filter(|(c, _)| *c == "event")
        .map(|(_, t)| J::Str(t.clone()))
        .collect();
    let stdout = expected
        .observations
        .iter()
        .filter(|(c, _)| *c == "stdout")
        .map(|(_, t)| J::Str(t.clone()))
        .collect();
    assert_eq!(doc.at("events"), &J::Arr(events));
    assert_eq!(doc.at("stdout"), &J::Arr(stdout));
    let bytes: usize = expected
        .observations
        .iter()
        .map(|(channel, text)| {
            let prefix = if *channel == "stdout" {
                ""
            } else if text.starts_with("open:")
                || text.starts_with("inspect:")
                || text.starts_with("close:")
            {
                "mock-resource:"
            } else {
                "mock-io:"
            };
            prefix.len() + text.len() + 1
        })
        .sum();
    assert_eq!(doc.at("observation_bytes"), &J::Num(bytes as i64));
    assert_eq!(doc.at("reserved_observation_bytes"), &J::Num(0));
    assert_eq!(doc.at("diagnostics"), &J::Arr(vec![]));
}
fn environment(request: Option<Result<i64, i64>>, writes: &[Result<(), i64>]) -> String {
    let requests = request
        .map(|r| match r {
            Ok(n) => format!(r#"{{"ok":"{n}"}}"#),
            Err(n) => format!(r#"{{"err":"{n}"}}"#),
        })
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    let writes = writes
        .iter()
        .map(|r| match r {
            Ok(()) => r#"{"ok":null}"#.to_owned(),
            Err(n) => format!(r#"{{"err":"{n}"}}"#),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{writes}],"mock_request":[{requests}],"mock_reply":[]}}"#
    )
}
fn ex04_expected(
    request: Result<i64, i64>,
    writes: &[Result<(), i64>],
    early_break: bool,
) -> Expect {
    let mut e = Expect::new(0, "ok", None);
    let limit = match request {
        Err(n) => {
            e.exit = 1;
            e.kind = "err";
            e.error = Some(n);
            e.event(format!("request:err:{n}"));
            return e;
        }
        Ok(n) => {
            e.event(format!("request:ok:{n}"));
            n
        }
    };
    if !(0..=1000).contains(&limit) {
        e.exit = 1;
        e.kind = "err";
        e.error = Some(41);
        return e;
    }
    e.event("open:0:4");
    // Mathematical recurrence, prepared before the CLI is invoked.
    let (mut square, mut odd) = (0, 1);
    for i in 0..limit as usize {
        e.event("inspect:0:4");
        match writes.get(i).copied().unwrap_or(Err(-9000)) {
            Ok(()) => {
                e.event(format!("write_i64:{square}:ok"));
                e.stdout(square.to_string());
            }
            Err(n) => {
                e.event(format!("write_i64:{square}:err:{n}"));
                e.exit = 1;
                e.kind = "err";
                e.error = Some(n);
                break;
            }
        }
        if early_break {
            break;
        }
        square += odd;
        odd += 2;
    }
    e.event("close:0:4");
    e
}

fn run_ex04(
    fixture: &Fixture,
    request: Result<i64, i64>,
    writes: &[Result<(), i64>],
    early_break: bool,
) {
    let expected = ex04_expected(request, writes, early_break);
    let env = fixture.env(&environment(Some(request), writes));
    assert_report(
        fixture.report("run", PROFILE, Some(&env)),
        PROFILE,
        &expected,
    );
}

#[test]
fn ex04_variable_limits_and_fifo_writes() {
    let fixture = Fixture::new(EX04);
    fixture.check(PROFILE);
    for limit in [0, 1, 2, 3, 4, 5, 17, 1000] {
        let writes = vec![Ok(()); limit as usize];
        run_ex04(&fixture, Ok(limit), &writes, false);
    }
    for limit in [-1, 1001] {
        run_ex04(&fixture, Ok(limit), &[], false);
    }
    // The two literal §11 byte totals remain explicit alongside the A2 formula.
    let success = ex04_expected(Ok(3), &[Ok(()); 3], false);
    let bytes: usize = success
        .observations
        .iter()
        .map(|(channel, text)| {
            let prefix = if *channel == "stdout" {
                ""
            } else if text.starts_with("open:")
                || text.starts_with("inspect:")
                || text.starts_with("close:")
            {
                "mock-resource:"
            } else {
                "mock-io:"
            };
            prefix.len() + text.len() + 1
        })
        .sum();
    assert_eq!(bytes, 221);
}

#[test]
fn ex04_typed_errors_close_resource_once() {
    let fixture = Fixture::new(EX04);
    fixture.check(PROFILE);
    run_ex04(&fixture, Err(10), &[], false);
    run_ex04(&fixture, Ok(3), &[Ok(()), Err(40), Ok(())], false);
    run_ex04(&fixture, Ok(1), &[], false);
    run_ex04(&fixture, Ok(1), &[Ok(())], false);
    let (exit, doc) = fixture.report("run", PROFILE, None);
    assert_report((exit, doc), PROFILE, &{
        let mut e = Expect::new(1, "err", Some(-9000));
        e.event("request:err:-9000");
        e
    });
}

#[test]
fn ex04_break_discards_suspended_generator() {
    let source = EX04.replace("MockWriteI64(valor)?;", "MockWriteI64(valor)?;break;");
    assert_eq!(source.matches("break;").count(), 1);
    let fixture = Fixture::new(&source);
    fixture.check(PROFILE);
    run_ex04(&fixture, Ok(3), &[Ok(()); 3], true);
    let expected = ex04_expected(Ok(3), &[Ok(()); 3], true);
    let bytes: usize = expected
        .observations
        .iter()
        .map(|(channel, text)| {
            let prefix = if *channel == "stdout" {
                ""
            } else if text.starts_with("open:")
                || text.starts_with("inspect:")
                || text.starts_with("close:")
            {
                "mock-resource:"
            } else {
                "mock-io:"
            };
            prefix.len() + text.len() + 1
        })
        .sum();
    assert_eq!(bytes, 119);
}

#[test]
fn ex04_overflow_reports_exact_runtime_diagnostic() {
    let fixture = Fixture::new(EX04_R);
    fixture.check(PROFILE);
    let env = fixture.env(&environment(Some(Ok(2)), &[Ok(()), Ok(())]));
    let (exit, doc) = fixture.report("run", PROFILE, Some(&env));
    assert_eq!(exit, 1);
    assert_eq!(
        doc.at("outcome"),
        &J::Obj(BTreeMap::from([(
            "kind".into(),
            J::Str("simulator_fault".into())
        )]))
    );
    assert_eq!(
        doc.at("observations"),
        &J::Arr(vec![
            observation("event", "request:ok:2"),
            observation("event", "open:0:4"),
            observation("event", "inspect:0:4"),
            observation("event", "write_i64:0:ok"),
            observation("stdout", "0"),
            observation("event", "close:0:4")
        ])
    );
    assert_eq!(
        doc.at("events"),
        &J::Arr(
            [
                "request:ok:2",
                "open:0:4",
                "inspect:0:4",
                "write_i64:0:ok",
                "close:0:4"
            ]
            .map(|s| J::Str(s.into()))
            .to_vec()
        )
    );
    assert_eq!(doc.at("stdout"), &J::Arr(vec![J::Str("0".into())]));
    assert_eq!(doc.at("observation_bytes"), &J::Num(119));
    assert_eq!(doc.at("reserved_observation_bytes"), &J::Num(0));
    let diagnostics = doc.at("diagnostics").array();
    assert_eq!(diagnostics.len(), 1);
    let d = &diagnostics[0];
    assert_eq!(d.at("schema_version"), &J::Num(1));
    assert_eq!(d.at("diagnostic_namespace"), &J::Str("F0-0002-sim".into()));
    assert_eq!(d.at("profile_id"), &J::Str(PROFILE.into()));
    assert_eq!(d.at("code"), &J::Str("R0101".into()));
    assert_eq!(d.at("subcause"), &J::Null);
    assert_eq!(d.at("rule"), &J::Str("R0101".into()));
    assert_eq!(d.at("explanation_id"), &J::Null);
    assert_eq!(d.at("severity"), &J::Str("error".into()));
    assert_eq!(d.at("stage"), &J::Str("runtime".into()));
    assert_eq!(
        d.at("message"),
        &J::Str("Desbordamiento de suma i64.".into())
    );
    assert_eq!(d.at("operation"), &J::Str("value.add".into()));
    assert_eq!(
        d.at("primary_span").at("file"),
        &J::Str(fixture.source.to_str().unwrap().into())
    );
    assert_eq!(d.at("primary_span").at("start"), &J::Num(234));
    assert_eq!(d.at("primary_span").at("end"), &J::Num(255));
    for field in ["related_spans", "cause_chain", "repair"] {
        assert_eq!(d.at(field), &J::Arr(vec![]));
    }
    for field in [
        "feature_id",
        "unexpected_token",
        "expected_tokens_kind",
        "parser_context",
        "recognized_prefix",
    ] {
        assert_eq!(d.at(field), &J::Null);
    }
    assert_eq!(d.at("expected_tokens"), &J::Arr(vec![]));
    assert_eq!(d.at("data"), &J::Obj(BTreeMap::new()));
    assert_eq!(
        EX04_R.as_bytes().get(234..255),
        Some(b"9223372036854775807+1".as_slice())
    );
    let positive = Fixture::new(EX04);
    positive.check(PROFILE);
    run_ex04(&positive, Ok(2), &[Ok(()); 2], false);
}
fn observation(channel: &str, text: &str) -> J {
    J::Obj(BTreeMap::from([
        ("channel".into(), J::Str(channel.into())),
        ("text".into(), J::Str(text.into())),
    ]))
}

#[test]
fn ex04_transport_errors_and_text_output() {
    let fixture = Fixture::new(EX04);
    fixture.check(PROFILE);
    let bad = fixture.env("{bad");
    let schema = fixture.dir.join("schema.json");
    fs::write(&schema, "{}").unwrap();
    let unreadable = fixture.dir.join("missing.json");
    let excessive = fixture.dir.join("excessive.json");
    fs::write(&excessive, vec![b' '; 2_000_000]).unwrap();
    for (path, reason) in [
        (&bad, "invalid_mock_json"),
        (&schema, "invalid_mock_schema"),
        (&unreadable, "mock_env_unreadable"),
        (&excessive, "mock_env_too_large"),
    ] {
        let (exit, doc) = fixture.report("run", PROFILE, Some(path));
        assert_eq!(exit, 2, "{reason}");
        assert_eq!(doc.at("outcome").at("kind"), &J::Str("driver_error".into()));
        assert_eq!(doc.at("outcome").at("reason"), &J::Str(reason.into()));
        empty_report(&doc);
    }
    let check_with_env = fixture.invoke("check", PROFILE, "json", Some(&bad));
    assert_eq!(check_with_env.status.code(), Some(2));
    assert_eq!(check_with_env.stderr, b"nexo-driver:invalid_arguments\n");
    assert!(check_with_env.stdout.is_empty());
    let env = fixture.env(&environment(Some(Ok(3)), &[Ok(()); 3]));
    let output = fixture.invoke("run", PROFILE, "text", Some(&env));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"0\n1\n4\n");
    let events = [
        "request:ok:3",
        "open:0:4",
        "inspect:0:4",
        "write_i64:0:ok",
        "inspect:0:4",
        "write_i64:1:ok",
        "inspect:0:4",
        "write_i64:4:ok",
        "close:0:4",
    ];
    let mut stderr = String::new();
    for event in events {
        let prefix = if event.starts_with("open:")
            || event.starts_with("inspect:")
            || event.starts_with("close:")
        {
            "mock-resource:"
        } else {
            "mock-io:"
        };
        stderr.push_str(prefix);
        stderr.push_str(event);
        stderr.push('\n');
    }
    stderr.push_str("nexo-result:Ok(())\n");
    assert_eq!(String::from_utf8(output.stderr).unwrap(), stderr);
}

#[test]
fn prior_normative_flows_remain_visible_in_both_profiles() {
    for profile in [EARLIER, PROFILE] {
        let f = Fixture::new(FLOW1);
        f.check(profile);
        type FlowCase = (
            &'static str,
            Result<(), i64>,
            &'static [&'static str],
            &'static [&'static str],
        );
        let cases: [FlowCase; 6] = [
            (
                r#"[{"ok":"12"},{"ok":"8"},{"ok":"5"}]"#,
                Ok(()),
                &["25"],
                &[
                    "open:0:7",
                    "read_records:0:ok:3",
                    "write_i64:25:ok",
                    "close:0:7",
                ],
            ),
            (
                "[]",
                Ok(()),
                &["0"],
                &[
                    "open:0:7",
                    "read_records:0:ok:0",
                    "write_i64:0:ok",
                    "close:0:7",
                ],
            ),
            (
                "ERR10",
                Err(10),
                &[],
                &["open:0:7", "read_records:0:err:10", "close:0:7"],
            ),
            (
                r#"[{"ok":"12"},{"err":"20"},{"ok":"5"}]"#,
                Err(20),
                &[],
                &["open:0:7", "read_records:0:ok:3", "close:0:7"],
            ),
            (
                r#"[{"ok":"9223372036854775807"},{"ok":"1"}]"#,
                Err(30),
                &[],
                &["open:0:7", "read_records:0:ok:2", "close:0:7"],
            ),
            (
                "WRITE40",
                Err(40),
                &[],
                &[
                    "open:0:7",
                    "read_records:0:ok:1",
                    "write_i64:12:err:40",
                    "close:0:7",
                ],
            ),
        ];
        for (records, outcome, stdout, events) in cases {
            let reads = if records == "ERR10" {
                r#"{"err":"10"}"#.to_owned()
            } else if records == "WRITE40" {
                r#"{"ok":[{"ok":"12"}]}"#.to_owned()
            } else {
                format!(r#"{{"ok":{records}}}"#)
            };
            let write = if records == "WRITE40" {
                r#"{"err":"40"}"#
            } else {
                r#"{"ok":null}"#
            };
            let json = format!(
                r#"{{"schema_version":1,"mock_read_records":[{reads}],"mock_write_i64":[{write}],"mock_request":[],"mock_reply":[]}}"#
            );
            let env = f.env(&json);
            let mut expected = Expect::new(
                if outcome.is_ok() { 0 } else { 1 },
                if outcome.is_ok() { "ok" } else { "err" },
                outcome.err(),
            );
            for event in events {
                expected.event(*event);
                if event.starts_with("write_i64:") && event.ends_with(":ok") {
                    expected.stdout(stdout[0]);
                }
            }
            assert_report(f.report("run", profile, Some(&env)), profile, &expected);
        }
        let f = Fixture::new(FLOW2);
        f.check(profile);
        assert_report(
            f.report("run", profile, None),
            profile,
            &Expect::new(0, "unit", None),
        );
        let f = Fixture::new(FLOW3);
        f.check(profile);
        for (request, reply, outcome, events) in [
            (Ok(0), Ok(()), Ok(()), vec!["request:ok:0", "reply:200:ok"]),
            (Ok(4), Ok(()), Ok(()), vec!["request:ok:4", "reply:404:ok"]),
            (Err(10), Ok(()), Err(10), vec!["request:err:10"]),
            (
                Ok(0),
                Err(40),
                Err(40),
                vec!["request:ok:0", "reply:200:err:40"],
            ),
        ] {
            let req = match request {
                Ok(n) => format!(r#"{{"ok":"{n}"}}"#),
                Err(n) => format!(r#"{{"err":"{n}"}}"#),
            };
            let rep = match reply {
                Ok(()) => r#"{"ok":null}"#.to_owned(),
                Err(n) => format!(r#"{{"err":"{n}"}}"#),
            };
            let json = format!(
                r#"{{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{req}],"mock_reply":[{rep}]}}"#
            );
            let env = f.env(&json);
            let mut expected = Expect::new(
                if outcome.is_ok() { 0 } else { 1 },
                if outcome.is_ok() { "ok" } else { "err" },
                outcome.err(),
            );
            for event in events {
                expected.event(event);
            }
            assert_report(f.report("run", profile, Some(&env)), profile, &expected);
        }
    }
}

#[test]
fn independent_static_negatives_match_check_and_run() {
    for (source, code, subcause, start, end) in [
        (
            "gen fn G()->I64{return 1;}fn Main(){}",
            "NX-E1324",
            None,
            16,
            22,
        ),
        (
            "fn Main(){break;}",
            "NX-E1326",
            Some("break_outside_loop"),
            10,
            15,
        ),
        (
            "fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=o?;return Ok(());}",
            "NX-E0102",
            Some("type_mismatch"),
            59,
            60,
        ),
    ] {
        let f = Fixture::new(source);
        let (check_exit, check) = f.report("check", PROFILE, None);
        let (run_exit, run) = f.report("run", PROFILE, None);
        assert_eq!((check_exit, run_exit), (1, 1));
        let a = check.at("diagnostics").array();
        let b = run.at("diagnostics").array();
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].at("code"), &J::Str(code.into()));
        assert_eq!(
            a[0].at("subcause"),
            &subcause.map_or(J::Null, |s| J::Str(s.into()))
        );
        assert_eq!(
            a[0].at("primary_span").at("file"),
            &J::Str(f.source.to_str().unwrap().into())
        );
        assert_eq!(a[0].at("primary_span").at("start"), &J::Num(start));
        assert_eq!(a[0].at("primary_span").at("end"), &J::Num(end));
    }
}
