"""Reference application and simulated environment v2 of the F4 oracle (class MODELO).

K 6.1 defines the environment (schema 2: seven queues, ``console_read_line`` and
``console_println``), K 10.6 the controller errors (E01-E07) and K 11 the reference application
(81-line source, eleven variants, one hundred lines).  Everything here is derived from those
sections:

* ``parse_env`` is the oracle's own strict JSON reader plus the schema-2 validation; it is the
  only place that decides ``invalid_mock_json`` or ``invalid_mock_schema``.
* ``run_app`` implements the business rule of K 11 with unbounded integers over a ``Console``
  (queue consumption, events, observation order and byte counts of K 6.1); it never executes
  Nexo text and never looks at the source of the application.
* ``app_records`` / ``env_records`` write the generated corpus classes A and E.

What K does not fix is NOT modelled (``ModelGap`` / LAGUNAS LF-22, LF-23): nesting deeper than
16, duplicate keys, raw control characters in a JSON string, the 1 MiB boundary and non-empty
legacy queues.  No generated environment contains them.
"""

from __future__ import annotations

import random

from . import charges, fixtures_io as fio, utf8ref
from .generate import PROFILE, _doc as result_doc, checked_doc, sha_hex
from .model import from_i64, parse_i64

APP_FILE = "total-registros.n"
KEYS = ("schema_version", "mock_read_records", "mock_write_i64", "mock_request", "mock_reply",
        "console_read_line", "console_println")
MAX_LINES = 100
I64_MIN, I64_MAX = -(1 << 63), (1 << 63) - 1
INVALID_JSON, INVALID_SCHEMA = "invalid_mock_json", "invalid_mock_schema"


class EnvError(Exception):
    """A controller error (exit 2): ``reason`` is ``invalid_mock_json`` or ``invalid_mock_schema``."""

    def __init__(self, reason: str, detail: str = ""):
        super().__init__(reason + (": " + detail if detail else ""))
        self.reason = reason


class ModelGap(Exception):
    """K does not fix the behaviour: the oracle never generates or compares it (LAGUNAS)."""


# ------------------------------------------------------------------ environment value

class Env:
    """``reads``: ("ok", text) | ("eof",) | ("err", n); ``writes``: ("ok",) | ("err", n)."""

    def __init__(self, reads=(), writes=()):
        self.reads = list(reads)
        self.writes = list(writes)


def _jstr(text: str) -> str:
    out = ['"']
    for ch in text:
        c = ord(ch)
        if ch == '"':
            out.append('\\"')
        elif ch == "\\":
            out.append("\\\\")
        elif c < 0x20:
            out.append("\\u%04x" % c)
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def read_value(item):
    kind = item[0]
    if kind == "ok":
        return {"ok": item[1]}
    if kind == "eof":
        return {"ok": None}
    return {"err": from_i64(item[1]).decode("ascii")}


def write_value(item):
    return {"ok": None} if item[0] == "ok" else {"err": from_i64(item[1]).decode("ascii")}


def env_tree(env: Env) -> dict:
    tree = {"schema_version": 2}
    for key in KEYS[1:5]:
        tree[key] = []
    tree["console_read_line"] = [read_value(i) for i in env.reads]
    tree["console_println"] = [write_value(i) for i in env.writes]
    return tree


def serialize(value, style=0, level=0) -> str:
    """JSON text of a tree (dict/list/str/None/bool/int).  ``style`` 0 is the compact form of K;
    1 adds spaces after ``:`` and ``,``; 2 is indented over several lines."""
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return from_i64(value).decode("ascii")
    if isinstance(value, str):
        return _jstr(value)
    if style == 2:
        pad, inner = "\n" + "  " * (level + 1), "\n" + "  " * level
    else:
        pad = inner = ""
    sep = ", " if style == 1 else ","
    colon = ": " if style in (1, 2) else ":"
    if isinstance(value, list):
        if not value:
            return "[]"
        body = (("," + pad) if style == 2 else sep).join(serialize(v, style, level + 1) for v in value)
        return "[" + pad + body + inner + "]"
    items = [_jstr(k) + colon + serialize(v, style, level + 1) for k, v in value.items()]
    if not items:
        return "{}"
    return "{" + pad + (("," + pad) if style == 2 else sep).join(items) + inner + "}"


def env_bytes(env: Env, rng=None) -> bytes:
    """Canonical environment bytes (the compact form of K 6.1); with ``rng`` the key order is
    shuffled and the layout varied (K: the key order is irrelevant)."""
    tree = env_tree(env)
    style = 0
    if rng is not None:
        keys = list(tree)
        rng.shuffle(keys)
        tree = {k: tree[k] for k in keys}
        style = rng.choice([0, 0, 1, 2])
    return utf8ref.text(serialize(tree, style))


# ------------------------------------------------------------------ strict JSON reader

WS = (0x20, 0x09, 0x0A, 0x0D)


class _Reader:
    def __init__(self, cps):
        self.c = cps
        self.i = 0

    def peek(self):
        return self.c[self.i] if self.i < len(self.c) else None

    def ws(self):
        while self.i < len(self.c) and self.c[self.i] in WS:
            self.i += 1

    def bad(self, why):
        raise EnvError(INVALID_JSON, why)

    def lit(self, word, value):
        for ch in word:
            if self.peek() != ord(ch):
                self.bad("literal")
            self.i += 1
        return value

    def value(self, depth):
        self.ws()
        ch = self.peek()
        if ch is None:
            self.bad("end")
        if ch == 0x7B:
            return self.obj(depth + 1)
        if ch == 0x5B:
            return self.arr(depth + 1)
        if ch == 0x22:
            return self.string()
        if ch == 0x74:
            return self.lit("true", True)
        if ch == 0x66:
            return self.lit("false", False)
        if ch == 0x6E:
            return self.lit("null", None)
        if ch == 0x2D or 0x30 <= ch <= 0x39:
            return self.number()
        self.bad("value")

    def number(self):
        start = self.i
        if self.peek() == 0x2D:
            self.i += 1
        d = self.peek()
        if d == 0x30:
            self.i += 1
        elif d is not None and 0x31 <= d <= 0x39:
            while self.peek() is not None and 0x30 <= self.peek() <= 0x39:
                self.i += 1
        else:
            self.bad("number")
        if self.peek() == 0x2E:
            self.i += 1
            if not (self.peek() is not None and 0x30 <= self.peek() <= 0x39):
                self.bad("fraction")
            while self.peek() is not None and 0x30 <= self.peek() <= 0x39:
                self.i += 1
        if self.peek() in (0x65, 0x45):
            self.i += 1
            if self.peek() in (0x2B, 0x2D):
                self.i += 1
            if not (self.peek() is not None and 0x30 <= self.peek() <= 0x39):
                self.bad("exponent")
            while self.peek() is not None and 0x30 <= self.peek() <= 0x39:
                self.i += 1
        return ("num", "".join(chr(x) for x in self.c[start:self.i]))

    def hex4(self):
        if self.i + 4 > len(self.c):
            self.bad("unicode escape")
        v = 0
        for k in range(4):
            h = self.c[self.i + k]
            if 0x30 <= h <= 0x39:
                d = h - 0x30
            elif 0x41 <= h <= 0x46:
                d = h - 0x41 + 10
            elif 0x61 <= h <= 0x66:
                d = h - 0x61 + 10
            else:
                self.bad("unicode escape")
            v = v * 16 + d
        self.i += 4
        return v

    def string(self):
        self.i += 1
        out = []
        while True:
            ch = self.peek()
            if ch is None:
                self.bad("unterminated string")
            self.i += 1
            if ch == 0x22:
                return "".join(out)
            if ch < 0x20:
                raise ModelGap("raw control character in a JSON string")
            if ch != 0x5C:
                out.append(chr(ch))
                continue
            e = self.peek()
            self.i += 1
            simple = {0x22: '"', 0x5C: "\\", 0x2F: "/", 0x62: "\b", 0x66: "\f", 0x6E: "\n", 0x72: "\r",
                      0x74: "\t"}
            if e in simple:
                out.append(simple[e])
            elif e == 0x75:
                v = self.hex4()
                if 0xDC00 <= v <= 0xDFFF:
                    self.bad("lone low surrogate")
                if 0xD800 <= v <= 0xDBFF:
                    if self.peek() != 0x5C or self.i + 1 >= len(self.c) or self.c[self.i + 1] != 0x75:
                        self.bad("lone high surrogate")
                    self.i += 2
                    w = self.hex4()
                    if not 0xDC00 <= w <= 0xDFFF:
                        self.bad("lone high surrogate")
                    v = 0x10000 + ((v - 0xD800) << 10) + (w - 0xDC00)
                out.append(chr(v))
            else:
                self.bad("escape")

    def arr(self, depth):
        if depth > 16:
            raise ModelGap("nesting deeper than 16")
        self.i += 1
        out = []
        self.ws()
        if self.peek() == 0x5D:
            self.i += 1
            return out
        while True:
            out.append(self.value(depth))
            self.ws()
            ch = self.peek()
            self.i += 1
            if ch == 0x5D:
                return out
            if ch != 0x2C:
                self.bad("array")

    def obj(self, depth):
        if depth > 16:
            raise ModelGap("nesting deeper than 16")
        self.i += 1
        out = {}
        self.ws()
        if self.peek() == 0x7D:
            self.i += 1
            return out
        while True:
            self.ws()
            if self.peek() != 0x22:
                self.bad("key")
            key = self.string()
            self.ws()
            if self.peek() != 0x3A:
                self.bad("colon")
            self.i += 1
            if key in out:
                raise ModelGap("duplicate key")
            out[key] = self.value(depth)
            self.ws()
            ch = self.peek()
            self.i += 1
            if ch == 0x7D:
                return out
            if ch != 0x2C:
                self.bad("object")


def parse_json(data: bytes):
    try:
        cps = utf8ref.decode(data)
    except utf8ref.Utf8Error:
        raise EnvError(INVALID_JSON, "utf-8")
    reader = _Reader(cps)
    value = reader.value(0)
    reader.ws()
    if reader.peek() is not None:
        reader.bad("trailing characters")
    return value


# ------------------------------------------------------------------ schema 2 validation

def canonical_i64(text) -> bool:
    """K 6.1 / A2: ``0`` or an optional ``-`` and digits, no ``+``, no leading zero, no ``-0``, in range."""
    if not isinstance(text, str) or not text:
        return False
    body = text[1:] if text[0] == "-" else text
    if not body or any(c < "0" or c > "9" for c in body) or (body[0] == "0" and text != "0"):
        return False
    return I64_MIN <= int(text) <= I64_MAX


def _schema(why):
    raise EnvError(INVALID_SCHEMA, why)


def validate(tree) -> Env:
    if not isinstance(tree, dict) or set(tree) != set(KEYS):
        _schema("keys")
    if tree["schema_version"] != ("num", "2"):
        _schema("schema_version")
    for key in KEYS[1:5]:
        if not isinstance(tree[key], list):
            _schema(key)
        if tree[key]:
            raise ModelGap("legacy queue with items")
    reads, writes = [], []
    if not isinstance(tree["console_read_line"], list) or not isinstance(tree["console_println"], list):
        _schema("queue type")
    for item in tree["console_read_line"]:
        if not isinstance(item, dict) or len(item) != 1:
            _schema("read item")
        if "ok" in item:
            v = item["ok"]
            if v is None:
                reads.append(("eof",))
            elif isinstance(v, str) and "\r" not in v and "\n" not in v:
                reads.append(("ok", v))
            else:
                _schema("ok payload")
        elif "err" in item and canonical_i64(item["err"]):
            reads.append(("err", int(item["err"])))
        else:
            _schema("read item")
    for item in tree["console_println"]:
        if not isinstance(item, dict) or len(item) != 1:
            _schema("write item")
        if "ok" in item and item["ok"] is None:
            writes.append(("ok",))
        elif "err" in item and canonical_i64(item["err"]):
            writes.append(("err", int(item["err"])))
        else:
            _schema("write item")
    return Env(reads, writes)


def parse_env(data: bytes) -> Env:
    """The whole controller validation of K 6.1 for the cases K fixes: JSON first, then schema 2."""
    return validate(parse_json(data))


# ------------------------------------------------------------------ console and observations

class Console:
    """Queue consumption of ReadLine/Println with the events and observation order of K 6.1."""

    def __init__(self, env: Env):
        self.reads = list(env.reads)
        self.writes = list(env.writes)
        self.order: list = []

    def event(self, text: str):
        self.order.append(("event", text))

    def read_line(self):
        item = self.reads.pop(0) if self.reads else ("err", -9000)
        if item[0] == "ok":
            self.event("read_line:ok:%d" % len(utf8ref.text(item[1])))
            return ("some", item[1])
        if item[0] == "eof":
            self.event("read_line:eof")
            return ("none",)
        self.event("read_line:err:%d" % item[1])
        return ("err", item[1])

    def println(self, text: str):
        item = self.writes.pop(0) if self.writes else ("err", -9000)
        if item[0] == "ok":
            self.event("println:ok:%d" % len(utf8ref.text(text)))
            self.order.append(("stdout", text))
            return ("ok",)
        self.event("println:err:%d" % item[1])
        return ("err", item[1])


class Run:
    def __init__(self, con: Console, kind: str, error=None):
        self.order = con.order
        self.kind = kind
        self.error = error

    @property
    def events(self):
        return [t for c, t in self.order if c == "event"]

    @property
    def stdout(self):
        return [t for c, t in self.order if c == "stdout"]

    @property
    def exit(self):
        return 0 if self.kind == "ok" else 1

    @property
    def observation_bytes(self):
        return sum((8 if c == "event" else 0) + len(utf8ref.text(t)) + 1 for c, t in self.order)

    def outcome(self) -> dict:
        if self.kind == "ok":
            return {"kind": "ok"}
        return {"kind": "err", "error": from_i64(self.error).decode("ascii")}

    def doc(self, profile=PROFILE) -> dict:
        return result_doc(profile, self.exit, self.outcome(), self.stdout, self.events, self.order, [], [])


def interleave(events, stdout):
    """Observation order of K 6.1 / 11: every successful println event is followed by its stdout."""
    out, k = [], 0
    for ev in events:
        out.append(("event", ev))
        if ev.startswith("println:ok:"):
            out.append(("stdout", stdout[k]))
            k += 1
    if k != len(stdout):
        raise ValueError("stdout does not match the println events")
    return out


# ------------------------------------------------------------------ the reference application (K 11)

class _Fail(Exception):
    def __init__(self, error):
        self.error = error


def convert(numero: int, text: str):
    """``Convertir`` of K 11: None for an empty line, else (numero, text, centimos); raises _Fail."""
    if text == "":
        return None
    tag, value = parse_i64(utf8ref.text(text))
    if tag == "Err":
        raise _Fail(value)
    if value < 0 or value > 10000:
        raise _Fail(42)
    base = value * 100
    if 10 <= value <= 100:
        base = base * 90 // 100
    return (numero, text, base + base * 21 // 100)


def summary_text(count: int, total: int) -> str:
    media = total // count if count != 0 else 0
    return "Registros: %d; total: %d EUR y %d cent; media: %d" % (count, total // 100, total % 100, media)


def run_app(env: Env) -> Run:
    con = Console(env)
    records = []
    try:
        for numero in range(1, MAX_LINES + 1):
            got = con.read_line()
            if got[0] == "err":
                raise _Fail(got[1])
            if got[0] == "none" or got[1] == "fin":
                break
            rec = convert(numero, got[1])
            if rec is not None:
                records.append(rec)
        total = 0
        for numero, text, centimos in records:
            total += centimos
            put = con.println("L%d: %s EUR => %d cent" % (numero, text, centimos))
            if put[0] == "err":
                raise _Fail(put[1])
        put = con.println(summary_text(len(records), total))
        if put[0] == "err":
            raise _Fail(put[1])
    except _Fail as exc:
        return Run(con, "err", exc.error)
    return Run(con, "ok")


def run_echo(env: Env) -> Run:
    """The program of K 10.6 (E01-E07): one ReadLine whose Err is propagated by ``?``."""
    con = Console(env)
    got = con.read_line()
    if got[0] == "err":
        return Run(con, "err", got[1])
    return Run(con, "ok")


def run_a08(env: Env) -> Run:
    """A08 (K 11.1): the interpolation ``"{"é"}:{if true then 1 else 2}"`` is printed with Println."""
    con = Console(env)
    put = con.println("é:1")
    if put[0] == "err":
        return Run(con, "err", put[1])
    return Run(con, "ok")


# ------------------------------------------------------------------ K environments

def base_env() -> Env:
    """K 11 'Entorno completo'."""
    return Env([("ok", "12"), ("ok", ""), ("ok", "8"), ("ok", "5"), ("eof",)], [("ok",)] * 4)


def full_hundred_env() -> Env:
    """K 11.2: one hundred lines ``1`` and the Err(70) sentinel; one hundred and one answers to println."""
    return Env([("ok", "1")] * 100 + [("err", 70)], [("ok",)] * 101)


def variant_env(name: str) -> Env:
    """The environments of the eleven variants of K 11: the base environment with one edit."""
    env = base_env()
    r, w = env.reads, env.writes
    if name == "18":
        r[2] = ("ok", "18")
    elif name == "eof_first":
        r[0] = ("eof",)
    elif name == "fin_first":
        r[0] = ("ok", "fin")
    elif name == "x_first":
        r[0] = ("ok", "x")
    elif name == "read_err_70":
        r[0] = ("err", 70)
    elif name == "minus_8":
        r[2] = ("ok", "-8")
    elif name == "10001":
        r[0] = ("ok", "10001")
    elif name == "overflow":
        r[0] = ("ok", "9223372036854775808")
    elif name == "write_err_first":
        w[0] = ("err", 71)
    elif name == "write_err_second":
        w[1] = ("err", 71)
    elif name == "empty_read_queue":
        env.reads = []
    else:
        raise KeyError(name)
    return env


VARIANT_NAMES = {"APP-V01": "18", "APP-V02": "eof_first", "APP-V03": "fin_first", "APP-V04": "x_first",
                 "APP-V05": "read_err_70", "APP-V06": "minus_8", "APP-V07": "10001", "APP-V08": "overflow",
                 "APP-V09": "write_err_first", "APP-V10": "write_err_second", "APP-V11": "empty_read_queue"}


# ------------------------------------------------------------------ random environments (class A)

GOOD_AMOUNTS = [0, 1, 5, 9, 10, 11, 50, 99, 100, 101, 500, 1000, 9999, 10000]
BAD_TEXTS = ["x", "1x", " 1", "1 ", "+1", "-", "١", "é", "1.5", "0x10", "1_0", "--1", "a1", "0,5"]
NEG_TEXTS = ["-1", "-8", "-10000", "-100"]
BIG_TEXTS = ["10001", "10002", "99999", "1000000", "9223372036854775807", "-9223372036854775808"]
OVERFLOW_TEXTS = ["9223372036854775808", "-9223372036854775809", "99999999999999999999", "1" + "0" * 25]
READ_ERRS = [70, -1, 5, 1000, -9000, I64_MAX, I64_MIN]


def _line(rng, mode):
    """One response of the read queue: (kind, item)."""
    t = rng.random()
    if mode == "clean":
        t *= 0.70
    if t < 0.50:
        n = rng.choice(GOOD_AMOUNTS) if rng.random() < 0.55 else rng.randint(0, 10000)
        text = str(n)
        if rng.random() < 0.08 and n < 1000:
            text = "0" * rng.randint(1, 3) + text
        if rng.random() < 0.03:
            text = "-0"
        return ("num", ("ok", text))
    if t < 0.62:
        return ("empty", ("ok", ""))
    if t < 0.66:
        return ("fin", ("ok", "fin"))
    if t < 0.70:
        return ("eof", ("eof",))
    if t < 0.76:
        return ("bad", ("ok", rng.choice(BAD_TEXTS)))
    if t < 0.82:
        return ("neg", ("ok", rng.choice(NEG_TEXTS)))
    if t < 0.88:
        return ("big", ("ok", rng.choice(BIG_TEXTS)))
    if t < 0.93:
        return ("ovf", ("ok", rng.choice(OVERFLOW_TEXTS)))
    if t < 0.97:
        return ("fin_like", ("ok", rng.choice(["Fin", "fin ", " fin", "FIN", "finx"])))
    return ("rerr", ("err", rng.choice(READ_ERRS)))


def random_env(rng) -> tuple:
    """(Env, shape) with 1 to 100 (rarely 101) lines; the shape is the sequence of line kinds."""
    mode = rng.choice(["clean"] * 5 + ["dirty"] * 2 + ["tiny"] * 2)
    n = rng.randint(1, 6) if mode == "tiny" else rng.choice([rng.randint(1, 100), rng.randint(1, 30),
                                                             100, 101, rng.randint(90, 100)])
    kinds, reads = [], []
    for _ in range(n):
        kind, item = _line(rng, "dirty" if mode == "tiny" else mode)
        kinds.append(kind)
        reads.append(item)
    if mode == "clean" and n < 100 and rng.random() < 0.8:
        end = rng.choice([("fin", ("ok", "fin")), ("eof", ("eof",))])
        kinds.append(end[0])
        reads.append(end[1])
    t = rng.random()
    wmode = "full" if t < 0.82 else ("err" if t < 0.92 else "short")
    if wmode == "full":
        writes = [("ok",)] * 101
        pattern = "w:full"
    elif wmode == "err":
        k = rng.randint(0, 4)
        writes = [("ok",)] * k + [("err", rng.choice([71, -2, 9, I64_MIN]))] + [("ok",)] * 3
        pattern = "w:err%d" % min(k, 3)
    else:
        writes = [("ok",)] * rng.randint(0, 5)
        pattern = "w:short"
    shape = "%s|%s>%s|n%d|%s" % (",".join(sorted(set(kinds))), kinds[0], kinds[-1], min(len(reads), 9), pattern)
    return Env(reads, writes), shape


# ------------------------------------------------------------------ environment faults (class E)

def _tree(rng) -> dict:
    reads = []
    for _ in range(rng.randint(0, 4)):
        kind = rng.random()
        if kind < 0.7:
            reads.append(("ok", rng.choice(["é", "a", "", "12", "x y", "🙂", "fin", "ñ" * 3])))
        elif kind < 0.85:
            reads.append(("eof",))
        else:
            reads.append(("err", rng.choice(READ_ERRS)))
    writes = [rng.choice([("ok",), ("ok",), ("err", 71)]) for _ in range(rng.randint(0, 3))]
    return env_tree(Env(reads, writes))


def _slot(rng, tree):
    """Index of a read response to damage: an existing one or a freshly appended one."""
    q = tree["console_read_line"]
    if not q or rng.random() < 0.3:
        q.append({"ok": rng.choice(["é", "a"])})
    return rng.randrange(len(q))


def _schema_fault(rng, tree, name):
    q, p = tree["console_read_line"], tree["console_println"]
    if name == "schema_one":
        tree["schema_version"] = 1
    elif name == "schema_three":
        tree["schema_version"] = 3
    elif name == "schema_string":
        tree["schema_version"] = "2"
    elif name == "schema_null":
        tree["schema_version"] = None
    elif name == "missing_key":
        del tree[rng.choice(KEYS)]
    elif name == "extra_key":
        tree["extra"] = rng.choice([[], 1, "x", None])
    elif name == "ok_int":
        q[_slot(rng, tree)] = {"ok": rng.choice([7, 0, -1, 12])}
    elif name == "ok_bool":
        q[_slot(rng, tree)] = {"ok": rng.choice([True, False])}
    elif name == "ok_array":
        q[_slot(rng, tree)] = {"ok": rng.choice([[], ["a"]])}
    elif name in ("ok_cr", "ok_lf", "ok_crlf"):
        i = _slot(rng, tree)
        base = rng.choice(["a", "é", "xy", "", "🙂z"])
        cut = rng.randint(0, len(base))
        ctl = {"ok_cr": "\r", "ok_lf": "\n", "ok_crlf": "\r\n"}[name]
        q[i] = {"ok": base[:cut] + ctl + base[cut:]}
    elif name == "err_int":
        q[_slot(rng, tree)] = {"err": rng.choice([7, 0, -3])}
    elif name == "err_noncanonical":
        q[_slot(rng, tree)] = {"err": rng.choice(["+1", "007", "-0", "", "1x", "9223372036854775808",
                                                  "-9223372036854775809", " 1", "1.0", "--1"])}
    elif name == "both_keys":
        q[_slot(rng, tree)] = {"ok": "a", "err": "1"}
    elif name == "empty_object":
        q[_slot(rng, tree)] = {}
    elif name == "unknown_key":
        q[_slot(rng, tree)] = {rng.choice(["fine", "OK", "Ok", "value"]): "a"}
    elif name == "not_object":
        q[_slot(rng, tree)] = rng.choice(["a", None, 3, []])
    elif name == "println_ok_int":
        p.append({"ok": rng.choice([1, "x", False])})
        rng.shuffle(p)
    elif name == "println_err_bad":
        p.append({"err": rng.choice(["x", "+1", "007", 5, ""])})
        rng.shuffle(p)
    elif name == "println_both":
        p.append({"ok": None, "err": "1"})
        rng.shuffle(p)
    elif name == "queue_not_array":
        tree[rng.choice(["console_read_line", "console_println"])] = rng.choice([{}, None, "x", 1])
    elif name == "root_array":
        return []
    else:
        raise KeyError(name)
    return tree


SCHEMA_FAULTS = ["schema_one", "schema_three", "schema_string", "schema_null", "missing_key", "extra_key",
                 "ok_int", "ok_bool", "ok_array", "ok_cr", "ok_lf", "ok_crlf", "err_int", "err_noncanonical",
                 "both_keys", "empty_object", "unknown_key", "not_object", "println_ok_int", "println_err_bad",
                 "println_both", "queue_not_array", "root_array"]
JSON_FAULTS = ["lone_high", "lone_low", "high_then_plain", "bad_utf8", "truncated", "trailing_comma",
               "trailing_garbage", "empty_file", "unquoted_key", "comment", "single_quotes"]
BAD_UTF8 = [b"\xc3\x28", b"\xff", b"\xc0\x80", b"\xe2\x82", b"\xed\xa0\x80", b"\xf4\x90\x80\x80", b"\x80",
            b"\xf0\x9f\x99"]
FAULT_REASON = {**{n: INVALID_SCHEMA for n in SCHEMA_FAULTS}, **{n: INVALID_JSON for n in JSON_FAULTS}}


def fault_env(rng, name: str) -> bytes:
    """Environment bytes damaged in exactly one way (``FAULT_REASON[name]`` is the reason)."""
    if name in SCHEMA_FAULTS:
        tree = _schema_fault(rng, _tree(rng), name)
        return utf8ref.text(serialize(tree, rng.choice([0, 1, 2])))
    tree = _tree(rng)
    if name in ("lone_high", "lone_low", "high_then_plain", "bad_utf8"):
        i = _slot(rng, tree)
        tree["console_read_line"][i] = {"ok": "a\u0001b"}
        text = utf8ref.text(serialize(tree, rng.choice([0, 1])))
        tag = rb"\u0001"
        if name == "lone_high":
            rep = (r"\ud%x%02x" % (8 + rng.randrange(4), rng.randrange(256))).encode()
        elif name == "lone_low":
            rep = (r"\ud%x%02x" % (12 + rng.randrange(4), rng.randrange(256))).encode()
        elif name == "high_then_plain":
            rep = rb"\ud800\u0041"
        else:
            rep = rng.choice(BAD_UTF8)
        assert tag in text
        return text.replace(tag, rep, 1)
    text = utf8ref.text(serialize(tree, rng.choice([0, 1])))
    if name == "truncated":
        last = text.rindex(b"}")
        return text[:rng.randint(1, last)]
    if name == "trailing_comma":
        k = text.rindex(b"]")
        return text[:k] + b"," + text[k:]
    if name == "trailing_garbage":
        return text + rng.choice([b"x", b"{}", b"0", b",", b"]"])
    if name == "empty_file":
        return rng.choice([b"", b" ", b"\n"])
    if name == "unquoted_key":
        return text.replace(b'"schema_version"', b"schema_version", 1)
    if name == "comment":
        return b"// c\n" + text
    if name == "single_quotes":
        return text.replace(b'"schema_version"', b"'schema_version'", 1)
    raise KeyError(name)


# ------------------------------------------------------------------ records (classes A and E)

def app_source() -> str:
    return next(e["source"] for e in fio.load_json("f4_oracle_literals.json")["entries"] if e["id"] == "APP")


def echo_source() -> str:
    return next(e["source"] for e in fio.load_json("f4_oracle_literals.json")["entries"] if e["id"] == "E01")


def make_record(seed, index, cls, family, source, env, expected, shape, role="main", file=APP_FILE,
                verbs=("check", "run")):
    exp = {"fields": []}
    exp.update(expected)
    return {"seed": seed, "index": index, "role": role, "cls": cls, "family": family, "fclass": cls,
            "phase": "G9", "file": file, "profile_id": PROFILE, "source_hex": source.encode("utf-8").hex(),
            "env_hex": env.hex(), "verbs": list(verbs), "expected": exp,
            "expected_sha256": fio.sha256_text(fio.dumps(exp)), "repr": None,
            "distinct": sha_hex(source.encode("utf-8") + b"\0" + env), "structural": sha_hex(shape)}


def app_records(seed: int, count: int, first_index: int = 0) -> list:
    """Class A: the fixed source of K 11 and a random environment of 1-100 lines."""
    source = app_source()
    rng = random.Random("f4o-app/%d" % seed)
    out, seen = [], set()
    attempts = 0
    while len(out) < count and attempts < count * 20:
        attempts += 1
        env, shape = random_env(rng)
        data = env_bytes(env, rng)
        if data in seen:
            continue
        seen.add(data)
        run = run_app(env)
        out.append(make_record(seed, first_index + len(out), "A", "app", source, data,
                               {"check": checked_doc(PROFILE), "run": run.doc()}, "A|" + shape))
    return out


def controller_doc(reason: str) -> dict:
    """Controller error of K 10.6 / A2: exit 2 and ``driver_error`` with the reason; only these two
    keys are compared (the rest of the document is not fixed by K)."""
    return {"exit": 2, "outcome": {"kind": "driver_error", "reason": reason}}


def env_records(seed: int, count: int, first_index: int = 0) -> list:
    """Class E: the program of K 10.6 with an environment that is valid or damaged in one way."""
    source = echo_source()
    rng = random.Random("f4o-env/%d" % seed)
    out, seen = [], set()
    names = SCHEMA_FAULTS + JSON_FAULTS
    turn = seed * 7
    attempts = 0
    while len(out) < count and attempts < count * 30:
        attempts += 1
        k = len(out)
        if k % 4 == 3:
            reads = rng.choice([[], [("ok", "é")], [("eof",)], [("err", 70)], [("ok", "a"), ("ok", "b")]])
            env = Env(reads, [])
            data = env_bytes(env, rng)
            family, shape = "env_valid", "E|valid|%d" % len(reads)
            expected = {"check": checked_doc(PROFILE), "run": run_echo(parse_env(data)).doc()}
            verbs = ("check", "run")
        else:
            name = names[turn % len(names)]
            turn += 1
            try:
                data = fault_env(rng, name)
            except (KeyError, ValueError):
                continue
            try:
                parse_env(data)
            except EnvError as exc:
                if exc.reason != FAULT_REASON[name]:
                    continue
                reason = exc.reason
            except ModelGap:
                continue
            else:
                continue
            family, shape = "env_" + name, "E|" + name
            expected = {"run": controller_doc(reason)}
            verbs = ("run",)
        if data in seen:
            continue
        seen.add(data)
        out.append(make_record(seed, first_index + len(out), "E", family, source, data, expected, shape,
                               file="env.n", verbs=verbs))
    return out


def special_records(seed: int, phase: str, closed, valid: int, invalid: int, final: bool) -> list:
    """Classes A (application), E (environment) and Q (quota recipes); they need K10 closed (G9, G10).
    Indexes start at 100000, 200000 and 300000 so that they never collide with V/N/L/B."""
    if phase not in ("G9", "G10") or "K10" not in closed:
        return []
    return (app_records(seed, valid // 10, 100000) + env_records(seed, invalid // 25, 200000)
            + charges.q_records(seed, final, 300000))
