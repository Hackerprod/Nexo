"""Self-test of the oracle (class ORQUESTA).

Order (plan 5): audit, pins and literal fragments, utf8ref against the host codec,
then the model groups G0..G8 under ``sys.addaudithook``.  Every expected value comes
from the pinned ADR (``fixtures_io``) or from the hand-written pairs; no binary is run.
"""

from __future__ import annotations

import functools
import io
import json
import os
import random
import re
import sys
import time

from . import audit, barrier, diagnostics, fixtures_io as fio, literal, utf8ref

ARNES_MESSAGE = "selftest"


class FailFast(BaseException):
    """Raised by the first failed check when ``--fail-fast`` is on (mutation campaigns)."""


class Report:
    def __init__(self, fail_fast: bool = False):
        self.lines: list[str] = []
        self.failed = 0
        self.total = 0
        self.fail_fast = fail_fast

    def check(self, name: str, ok: bool, detail: str = "") -> bool:
        self.total += 1
        if ok:
            self.lines.append(f"PASS {name}")
        else:
            self.failed += 1
            self.lines.append(f"FAIL {name}: {detail}")
            if self.fail_fast:
                raise FailFast(name)
        return ok


# ------------------------------------------------------------------ dynamic audit

class _Hook:
    active = False
    violations: list = []
    installed = False
    allowed_prefixes: list = []
    allowed_imports: set = set()


def _norm(path) -> str | None:
    if isinstance(path, bytes):
        path = path.decode("utf-8", "replace")
    if not isinstance(path, str):
        return None
    return os.path.normcase(os.path.abspath(path))


def _install_hook():
    if _Hook.installed:
        return
    # lazy imports of the standard library (hashing, seeding) must not happen while armed
    hashlib_warm = __import__("hashlib")
    hashlib_warm.sha256(b"").hexdigest()
    random.Random("warm").random()
    stdlib = os.path.dirname(os.path.abspath(os.__file__))
    prefixes = [os.path.normcase(p) for p in fio.allowed_read_paths()]
    prefixes.append(os.path.normcase(audit.HERE))
    prefixes.append(os.path.normcase(stdlib))
    for extra in (sys.prefix, sys.base_prefix):
        prefixes.append(os.path.normcase(os.path.abspath(os.path.join(extra, "Lib"))))
        prefixes.append(os.path.normcase(os.path.abspath(os.path.join(extra, "lib"))))
    _Hook.allowed_prefixes = prefixes
    _Hook.allowed_imports = (set(audit.MODEL_STDLIB) | audit.MODELO | audit.LECTOR | audit.ORQUESTA
                             | {"scripts", "scripts.f4_oracle"})

    def hook(event, args):
        if not _Hook.active:
            return
        if event == "open":
            norm = _norm(args[0])
            if norm is None:
                return
            if any(norm.startswith(p) for p in _Hook.allowed_prefixes):
                return
            _Hook.violations.append(f"open {norm}")
        elif event in ("subprocess.Popen", "os.system", "os.exec", "os.posix_spawn", "os.spawn"):
            _Hook.violations.append(event)
        elif event.startswith("socket.") or event.startswith("ctypes."):
            _Hook.violations.append(event)
        elif event == "import":
            name = args[0]
            top = name.split(".")[0]
            if name.startswith("scripts.f4_oracle."):
                top = name.split(".")[2]
            if top not in _Hook.allowed_imports and not name.startswith("encodings"):
                _Hook.violations.append(f"import {name}")

    sys.addaudithook(hook)
    _Hook.installed = True


def run_audited(fn):
    """Run ``fn`` with the audit hook armed; return (result, violations)."""
    _install_hook()
    _Hook.violations = []
    _Hook.active = True
    try:
        result = fn()
    finally:
        _Hook.active = False
    return result, list(_Hook.violations)


# ------------------------------------------------------------------ fixtures

_K = None


def ktext() -> str:
    global _K
    if _K is None:
        _K = fio.adr_text("K")
    return _K


def load_literals() -> list:
    return fio.load_json("f4_oracle_literals.json")["entries"]


def entry_map() -> dict:
    return {e["id"]: e for e in load_literals()}


def expected_diagnostic(entry: dict) -> dict:
    """Contract-C diagnostic for a literal row (built from the row's structured fields)."""
    e = entry["expected"]
    kw = {}
    if e.get("token"):
        t = e["token"]
        kw["unexpected_token"] = (t[0], t[1], (t[2], t[3]))
    if e.get("profile_id"):
        kw["profile_id"] = e["profile_id"]
    if e.get("stage"):
        kw["stage"] = e["stage"]
    if e.get("operation"):
        kw["operation"] = e["operation"]
    if e.get("feature_id"):
        kw["feature_id"] = e["feature_id"]
    if e.get("parser_context"):
        kw["parser_context"] = e["parser_context"]
    if e.get("recognized_prefix"):
        kw["recognized_prefix"] = tuple(e["recognized_prefix"])
    if e.get("expected_tokens"):
        kw["expected_tokens"] = e["expected_tokens"]
    if e.get("expected_tokens_kind"):
        kw["expected_tokens_kind"] = e["expected_tokens_kind"]
    chain = [(c[0], c[1], (c[2], c[3]), c[4]) for c in e.get("cause_chain", [])]
    related = [(r[0], (r[1], r[2])) + ((r[3],) if len(r) > 3 else ()) for r in e["related"]]
    d = diagnostics.make(e["code"], e["subcause"], file=entry["file"], primary=tuple(e["primary"]),
                         related=related, data=e["data"], cause_chain=chain, **kw)
    if e.get("message"):
        d["message"] = e["message"]
    return d


def diag_equal(a: dict, b: dict) -> list[str]:
    """Strict comparison of the 21 contract fields (message only when both carry it)."""
    out = []
    for name in diagnostics.FIELDS:
        if a.get(name) != b.get(name):
            out.append(name)
    if "message" in a and "message" in b and a["message"] != b["message"]:
        out.append("message")
    return out


# ------------------------------------------------------------------ core checks

def check_audit(rep: Report):
    findings = audit.run()
    rep.check("audit.static", not findings, "; ".join(findings[:5]))


def check_pins(rep: Report):
    pins = fio.load_json("f4_oracle_pins.json")
    for key, pin in pins["adr"].items():
        ok = (fio.adr_blob(key) == pin["blob"] and fio.adr_sha256(key) == pin["sha256_lf"])
        rep.check(f"pins.adr.{key}", ok, "ADR_CHANGED")
    rep.check("pins.K.pin", pins["adr"]["K"]["blob"] == "8bf37f56c9afd7c4d8c8bb212ea1b2b7b29635a5"
              and pins["adr"]["K"]["sha256_lf"] ==
              "cb9dc5e93d7965c99d871fbd386a6d5375ff443471b45194a88ae0901125d941", "K pin")
    bad = []
    for frag in pins["fragments"]:
        try:
            text = fio.extract(fio.adr_text(frag["source_doc"]), frag["section"], frag["locator"])
        except fio.FixtureError as exc:
            bad.append(f"{frag['id']}: {exc}")
            continue
        if text != frag["fragment"] or fio.sha256_text(text) != frag["sha256"]:
            bad.append(frag["id"])
    rep.check("pins.fragments", not bad, ", ".join(bad))


def check_literal_rows(rep: Report):
    entries = load_literals()
    bad_frag, bad_mark, bad_shape, bad_pos = [], [], [], []
    for e in entries:
        for frag in e["fragments"]:
            try:
                text = fio.extract(ktext(), frag["section"], frag["locator"])
            except fio.FixtureError as exc:
                bad_frag.append(f"{e['id']}: {exc}")
                continue
            if text != frag["text"]:
                bad_frag.append(e["id"])
        src = e["source"]
        if "marked" in e:
            m = e["marked"]
            a, b = m.index("⟦"), m.index("⟧")
            plain = m[:a] + m[a + 1:b] + m[b + 1:]
            span = [len(utf8ref.text(m[:a])), len(utf8ref.text(m[:a])) + len(utf8ref.text(m[a + 1:b]))]
            if plain != src or span != e["expected"]["primary"]:
                bad_mark.append(e["id"])
        if e["expected"]["kind"] == "diagnostic":
            d = expected_diagnostic(e)
            problems = diagnostics.check_shape(d)
            size = len(utf8ref.text(src))
            p = d["primary_span"]
            if not (0 <= p["start"] <= p["end"] <= size):
                problems.append("primary outside source")
            for r in d["related_spans"]:
                if r["span"]["file"] == e["file"] and not (0 <= r["span"]["start"] <= r["span"]["end"] <= size):
                    problems.append("related outside source")
            if problems:
                bad_shape.append(f"{e['id']}: {problems[0]}")
        pos = e.get("positive")
        if pos is not None and pos["source"] == src:
            bad_pos.append(e["id"])
    rep.check("literals.fragments", not bad_frag, ", ".join(bad_frag[:6]))
    rep.check("literals.marks", not bad_mark, ", ".join(bad_mark[:6]))
    rep.check("literals.shape", not bad_shape, ", ".join(bad_shape[:6]))
    rep.check("literals.positive_differs", not bad_pos, ", ".join(bad_pos[:6]))
    rep.check("literals.count", len(entries) >= 140, str(len(entries)))


def _k122_scalars() -> list[int]:
    lines = [ln for ln in section_lines_122() + fio.section_lines(ktext(), "10.7") if "U+" in ln]
    out: list[int] = []
    for ln in lines:
        for m in re.finditer(r"U\+([0-9A-Fa-f]{4,6})", ln):
            cp = int(m.group(1), 16)
            if cp not in out:
                out.append(cp)
    return out


def section_lines_122() -> list[str]:
    return fio.section_lines(ktext(), "12.2")


def check_utf8(rep: Report, fast: bool):
    rng = random.Random(4)
    scalars = _k122_scalars()
    rep.check("utf8ref.k122_scalars", len(scalars) >= 10 and 0x10FFFF in scalars and 0xD7FF in scalars,
              str(scalars))
    step = 97 if fast else 1
    bad = []
    for cp in range(0, 0x110000, step):
        if 0xD800 <= cp <= 0xDFFF:
            try:
                utf8ref.encode_scalar(cp)
                bad.append(("surrogate accepted", cp))
            except utf8ref.Utf8Error:
                pass
            continue
        host = chr(cp).encode("utf-8")
        mine = utf8ref.encode_scalar(cp)
        if host != mine or utf8ref.decode(mine) != [cp]:
            bad.append(("roundtrip", cp))
            break
    for cp in scalars:
        if utf8ref.encode_scalar(cp) != chr(cp).encode("utf-8"):
            bad.append(("k122", cp))
    rep.check("utf8ref.encode_decode_all_scalars", not bad, str(bad[:3]))
    invalid = [b"\x80", b"\xbf", b"\xc0\x80", b"\xc1\xbf", b"\xe0\x80\x80", b"\xf0\x80\x80\x80",
               b"\xed\xa0\x80", b"\xed\xbf\xbf", b"\xe2\x82", b"\xf0\x9f\x99", b"\xf4\x90\x80\x80",
               b"\xf5\x80\x80\x80", b"\xff", b"\xc3\x28", b"a\x80", b"\xe2\x28\xa1"]
    for _ in range(20000):
        invalid.append(bytes(rng.randrange(256) for _ in range(rng.randrange(0, 6))))
    mism = []
    for data in invalid:
        try:
            data.decode("utf-8")
            host_ok = True
        except UnicodeDecodeError:
            host_ok = False
        if host_ok != utf8ref.is_valid(data):
            mism.append(data.hex())
            continue
        if host_ok:
            same = utf8ref.decode(data) == [ord(c) for c in data.decode("utf-8")]
            if not same:
                mism.append(data.hex())
    rep.check("utf8ref.matches_host_validity", not mism, ", ".join(mism[:4]))
    # unsigned byte order against the host
    sample = [bytes(rng.randrange(256) for _ in range(rng.randrange(0, 5))) for _ in range(400)]
    sample += [chr(cp).encode("utf-8") for cp in scalars] + [b"", b"a", b"aa", "é".encode("utf-8")]
    bad_cmp = []
    for a in sample:
        for b in sample[:60]:
            want = (a > b) - (a < b)
            if utf8ref.compare(a, b) != want:
                bad_cmp.append((a.hex(), b.hex()))
    rep.check("utf8ref.compare_unsigned", not bad_cmp, str(bad_cmp[:2]))
    text = "é"
    rep.check("utf8ref.no_normalization",
              utf8ref.scalar_len(utf8ref.text(text)) == 2 and utf8ref.text(text) != utf8ref.text("é"))
    bounds = [i for i in range(0, 8) if utf8ref.is_boundary("\U0001F642".encode("utf-8"), i)]
    rep.check("utf8ref.boundaries", bounds == [0, 4], str(bounds))


def check_cause_evidence(rep: Report):
    """K 10.5 (X23, N06-N09, B01, P01): every causal step has evidence=verified, one of the three values of A2 13.6."""
    line = next(ln for ln in ktext().splitlines() if ln.startswith("Cause_chain, un paso por cada fila"))
    want = re.search(r"evidence=(\w+)", line).group(1)
    allowed = re.search(r'"evidence": ((?:"\w+"(?: \| )?)+)', fio.adr_text("A2"))
    values = re.findall(r'"(\w+)"', allowed.group(1))
    chains = [c for e in load_literals() if e["expected"]["kind"] == "diagnostic"
              for c in expected_diagnostic(e)["cause_chain"]]
    rep.check("K10.5.cause_evidence_is_verified",
              want == "verified" and want in values and chains and all(c["evidence"] == want for c in chains),
              f"{want} {values} {len(chains)}")


def check_diagnostic_contract(rep: Report):
    check_cause_evidence(rep)
    d = diagnostics.make("NX-E1401", "empty_interpolation", file="x.n", primary=(1, 2))
    rep.check("diagnostics.fields", diagnostics.check_shape(d) == [] and len(diagnostics.FIELDS) == 21)
    r = diagnostics.make("R0002", file="x.n", primary=(1, 2), data={"limit": 1})
    rep.check("diagnostics.r0002_identity", r["explanation_id"] is None and r["rule"] == "R0002"
              and r["diagnostic_namespace"] == "F0-0002-sim" and r["operation"] == "runtime.limit")


# ------------------------------------------------------------------ G0

G0_LITERAL_IDS = ["X01", "X02", "X03", "L01", "L02", "H01", "H02", "H03", "H04", "H05", "H06"]
G0_QUIET_IDS = ["L03", "L04", "L05"]


def _literal_diag(entry: dict, source: str):
    """NX-E1401 diagnostic produced by the literal analyser for ``source`` (or None)."""
    found = literal.first_diagnostic(source)
    if found is None or "other" in found:
        return None
    return diagnostics.make(
        "NX-E1401", found["subcause"], file=entry["file"], primary=found["primary"],
        related=[("delimiter", found["delimiter"])],
        unexpected_token=(found["token"][0], found["token"][1], found["token"][2]))


def run_g0(rep: Report):
    guards_literal(rep)
    emap = entry_map()
    for ident in G0_LITERAL_IDS:
        e = emap[ident]
        got = _literal_diag(e, e["source"])
        want = expected_diagnostic(e)
        diff = ["missing"] if got is None else diag_equal(got, want)
        rep.check(f"G0.literal.{ident}", not diff, ",".join(diff))
        pos = e.get("positive")
        if pos is not None:
            found = literal.first_diagnostic(pos["source"])
            rep.check(f"G0.literal.{ident}.positive", found is None, str(found))
    for ident in G0_QUIET_IDS + ["U01"]:
        e = emap[ident]
        rep.check(f"G0.literal.{ident}.no_syntax_error", literal.first_diagnostic(e["source"]) is None)
    # resume offsets named in K 2.1 (H01..H03)
    resume = {"H01": 25, "H02": 24, "H03": 24}
    for ident, want in resume.items():
        got = literal.first_diagnostic(emap[ident]["source"])["resume"]
        rep.check(f"G0.literal.{ident}.resume", got == want, str(got))
    # literals K leaves open must not be classified
    open_cases = {"escape": 'fn Main(){let s="\\q";}', "expr": 'fn Main(){let s="{1 + }";}',
                  "crude": 'fn Main(){let s="a\nb";}'}
    for name, src in open_cases.items():
        found = literal.first_diagnostic(src)
        rep.check(f"G0.literal.open.{name}", found is not None and "other" in found, str(found))
    # N13 and the barrier pairs
    e = emap["N13"]
    lits = literal.find_literals(e["source"])
    cons = barrier.literal_constructs(literal.to_bytes((lits[0].start, lits[0].end),
                                                       literal.byte_map(e["source"])), lits[0].holes)
    got = barrier.barrier_diagnostic(cons, {"K01"}, e["file"])
    want = expected_diagnostic(e)
    diff = ["missing"] if got is None else diag_equal(got, want)
    rep.check("G0.barrier.N13", not diff, ",".join(diff))
    rep.check("G0.barrier.N13.message", got is not None and got["message"] == want["message"])
    pos = barrier.barrier_diagnostic(cons, {"K01", "K02"}, e["file"])
    rep.check("G0.barrier.N13.closed", pos is None)
    pairs = fio.load_json("f4_oracle_model_pairs.json")
    for pair in pairs["barrier"]:
        exp = pair["expect"]
        src = pair["source"]
        found = literal.first_diagnostic(src)
        if found is not None and "other" not in found:
            ok = (exp["kind"] == "syntax" and found["subcause"] == exp["subcause"]
                  and list(found["primary"]) == exp["primary"])
            rep.check(f"G0.barrier.{pair['id']}", ok, str(found))
            continue
        cons = [barrier.Construct(c[0], (c[1], c[2])) for c in pair["constructs"]]
        got = barrier.barrier_diagnostic(cons, set(pair["closed"]), pair["file"])
        if exp["kind"] == "none":
            rep.check(f"G0.barrier.{pair['id']}", got is None)
            continue
        ok = got is not None and got["data"] == {"pending_scope": exp["pending_scope"]} \
            and got["feature_id"] == "implementation_limit" and got["code"] == "NX-E0006"
        if ok and exp["compare_primary"]:
            ok = [got["primary_span"]["start"], got["primary_span"]["end"]] == exp["primary"]
        rep.check(f"G0.barrier.{pair['id']}", ok, str(got and got["data"]))
    for pair in pairs["manifest"]:
        try:
            barrier.validate_manifest(pair["phase"], pair["complete"])
            valid = True
        except barrier.ManifestError:
            valid = False
        rep.check(f"G0.manifest.{pair['id']}", valid == pair["valid"])
    ktable = {c: barrier.CONSTRUCT_K[c] for c in barrier.CONSTRUCT_K}
    rep.check("G0.barrier.table_closed", set(ktable.values()) == {f"K{n:02d}" for n in range(2, 11)})


# ------------------------------------------------------------------ O2: model pairs G1-G8
# Each builder describes one K row as a typed program (never parsed from text).  ``pos`` is
# False for the negative fixture and True for its single-edit positive.

from . import model as m

I, B, S, N, C, lab, edit, move, M = m.I, m.B, m.S, m.N, m.C, m.lab, m.edit, m.move, m.M
FLD, V, BIN, ARR, LET, VAR, ASG, ES = m.F, m.V, m.BIN, m.ARR, m.LET, m.VAR, m.ASG, m.ES
FOR, ARM, MATCH, STRUCT, ENUM, FN, EXT, METH = m.FOR, m.ARM, m.MATCH, m.STRUCT, m.ENUM, m.FN, m.EXT, m.METH
Program, Use, UseExt, RawDecl, PVar, PBind, PWild, PNone, PSome = (
    m.Program, m.Use, m.UseExt, m.RawDecl, m.PVar, m.PBind, m.PWild, m.PNone, m.PSome)
IDX, TRY, IFE, Mv = m.IDX, m.TRY, m.IFE, m.Mv


def main(*body, ret=None, ml=False):
    return FN("Main", [], list(body), ret=ret, ml=ml)


def AEQ(a, b):
    return ES(C("AssertEq", a, b))


def prog(*decls, **kw):
    return Program(list(decls), **kw)


def _op_rows(ident, ops, build):
    return {f"{ident}-{op}": (lambda pos, op=op: build(pos, op)) for op in ops}


OPS6 = ["<", "<=", ">", ">=", "==", "!="]

# kind: check (static diagnostics), run (runtime fault), result (stdout/events), access (P04)
BUILD = {
    # ---- G1
    "X04": ("check", lambda pos: prog(main(LET("s", IDX(ARR(I(233)) if pos else S("é"), I(0)))))),
    "X23": ("check", lambda pos: prog(FN("F", [("s", "view", "String")],
                                         [ES(M(N("s"), "ByteLen"))] if pos else [LET("t", N("s"))],
                                         with_="no_alloc"))),
    "X24": ("run", lambda pos: prog(main(LET("s", S("abcdefgh" if pos else "abcdefghi")))),
            {"limits": {"string_bytes": 8}, "profile": "f0-0004-b-test-string-8-v1"}),
    "X25": ("run", lambda pos: prog(main(LET("a", S("12345678")), LET("b", S("12345678")),
                                         LET("c", S("" if pos else "x")))),
            {"limits": {"live_text_bytes": 16}, "profile": "f0-0004-b-test-text-live-16-v1"}),
    "X26": ("run", lambda pos: prog(main(LET("a", S("")), LET("b", S("")), LET("c", I(0) if pos else S("")))),
            {"limits": {"text_allocations": 2}, "profile": "f0-0004-b-test-text-alloc-2-v1"}),
    "N09": ("check", lambda pos: prog(FN("F", [], [LET("s", I(0) if pos else S("a"))], with_="no_alloc"))),
    # ---- G2
    "X22": ("check", lambda pos: prog(main(LET("s", S("a")), LET("t", M(N("s"), "ByteLen" if pos else "Nope"))))),
    "M05": ("check", lambda pos: prog(main(LET("s", S("a")), ES(M(N("s"), "Scalars" if pos else "Graphemes"))))),
    "O04": ("check", lambda pos: prog(main(LET("x", BIN(S("é"), "==" if pos else "+", S("x")))))),
    "O10": ("check", lambda pos: prog(main(FOR("x", ARR(S("é")) if pos else S("é"), [])))),
    "O11": ("check", lambda pos: prog(main(FOR("x", ARR(S("é")), [], edit_=not pos)))),
    "O13": ("run", lambda pos: prog(main(VAR("a", ARR(S("é"))), FOR("x", N("a"), [ES(M(N("x"), "ByteLen"))]),
                                         ES(C("Push", edit(N("a")), S("x") if pos else M(S("x"), "Concat", S("")))),
                                         ES(C("Pop", edit(N("a")))))),
            {"limits": {"text_allocations": 2}, "profile": "f0-0004-b-test-text-alloc-2-v1"}),
    "B01-text": ("check", lambda pos: prog(main(VAR("a", ARR(S("x"))),
                                                FOR("x", ARR(S("x")) if pos else N("a"),
                                                    [ASG(IDX(N("a"), I(0)), S("y"))])))),
    # ---- G3
    "X05": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      main(LET("s", C("S", I(1))), ES(FLD(N("s"), "x" if pos else "y"))))),
    "X06": ("check", lambda pos: prog(STRUCT("S", [("x", "I64"), ("y" if pos else "x", "I64")]))),
    "X07": ("check", lambda pos: prog(ENUM("E", [("A", []), ("B" if pos else "A", [])]))),
    "X08": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      main(LET("s", C("S", I(1)) if pos else C("S"))))),
    "X09": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      main(LET("s", C("S", lab("x" if pos else "z", I(1))))))),
    "X10": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      main((VAR if pos else LET)("s", C("S", I(1))),
                                           ASG(FLD(N("s"), "x"), I(2))))),
    "X21": ("check", lambda pos: prog(STRUCT("S", [("x", "I64" if pos else "S")]))),
    "X27": ("check", lambda pos: prog(STRUCT("S", [("a", "I64"), ("b", "I64")] + ([] if pos else [("c", "I64")]))),
            {"limits": {"nominal_members": 2, "profile_id": "f0-0004-b-test-members-2-v1"}}),
    "M07": ("check", lambda pos: prog(STRUCT("T", []) if pos else RawDecl("trait T{}", "trait"))),
    "N03": ("check", lambda pos: prog(ENUM("E", [("A", [])]),
                                      main(LET("e", V("E", "A" if pos else "B", null=True))))),
    "N04": ("check", lambda pos: prog(STRUCT("A", []), STRUCT("B", []),
                                      main(LET("x", C("A" if pos else "B"), t="A")))),
    "N12": ("check", lambda pos: prog(ENUM("E", [("A", [])] if pos else []))),
    "P02": ("check", lambda pos: prog(STRUCT("S", [("x", "Result<I64,I64>" if pos else "Result<S,I64>")]))),
    "P05": ("check", lambda pos: prog(ENUM("Evento", [("Inicio", [])]),
                                      main(ES(V("Evento", "Inicio", null=True, called=not pos))))),
    "P06": ("check", lambda pos: prog(STRUCT("S", [("campo", "I64")]),
                                      main(LET("p", C("S", I(1))),
                                           ES(C("Drop", N("p") if pos else FLD(N("p"), "campo")))))),
    "H09": ("check", lambda pos: prog(STRUCT("A", [("b", "B")]), STRUCT("B", [("a", "I64" if pos else "A")]))),
    "O12": ("check", lambda pos: prog(STRUCT("S", []), main(FOR("x", ARR(C("S")) if pos else C("S"), [])))),
    "B01-struct": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                             main(VAR("a", ARR(C("S", I(1)))),
                                                  FOR("x", ARR(C("S", I(1))) if pos else N("a"),
                                                      [ASG(IDX(N("a"), I(0)), C("S", I(2)))])))),
    "B01-enum": ("check", lambda pos: prog(ENUM("E", [("V", ["I64"])]),
                                           main(VAR("a", ARR(V("E", "V", I(1)))),
                                                FOR("x", ARR(V("E", "V", I(1))) if pos else N("a"),
                                                    [ASG(IDX(N("a"), I(0)), V("E", "V", I(2)))])))),
    # ---- G4
    "X11": ("check", lambda pos: prog(STRUCT("S", [("r", "MockResource")]),
                                      main(LET("s", C("S", C("MockOpen", I(1)))),
                                           LET("t", Mv(N("s")) if pos else N("s"))))),
    "M04": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      main(LET("s", C("S", I(1))),
                                           LET("x", FLD(N("s"), "x") if pos else Mv(FLD(N("s"), "x")))))),
    "N05": ("check", lambda pos: prog(ENUM("E", [("A", []), ("B", ["MockResource"])]),
                                      main(LET("e", V("E", "A", null=True)),
                                           LET("f", Mv(N("e")) if pos else N("e"))))),
    "N06": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      FN("F", [("s", "edit" if pos else "view", "S")],
                                         [ASG(FLD(N("s"), "x"), I(2))]))),
    "N07": ("check", lambda pos: prog(STRUCT("S", [("x", "I64"), ("y", "I64")]),
                                      FN("F", [("a", "edit", "I64"), ("b", "edit", "I64")], []),
                                      main(*([VAR("s", C("S", I(1), I(2))), VAR("t", C("S", I(3), I(4))),
                                              ES(C("F", edit(FLD(N("s"), "x")), edit(FLD(N("t"), "y"))))] if pos else
                                             [VAR("s", C("S", I(1), I(2))),
                                              ES(C("F", edit(FLD(N("s"), "x")), edit(FLD(N("s"), "y"))))])))),
    "N08": ("check", lambda pos: prog(STRUCT("S", []),
                                      main(*([LET("s", C("S"))] + ([] if pos else [ES(C("Drop", N("s")))])
                                             + [LET("t", N("s"))])))),
    "O09": ("check", lambda pos: prog(STRUCT("S", [("r", "I64" if pos else "MockResource")]),
                                      main(LET("a", ARR(), t="Array<S>")))),
    # ---- G5
    "M01": ("check", lambda pos: prog(STRUCT("S", []), EXT("S", METH("F", "edit")), UseExt("package.S"),
                                      main(VAR("s", C("S")), ES(M(N("s"), "F", prefix="edit" if pos else None))))),
    "M02": ("check", lambda pos: prog(*([STRUCT("S", []), EXT("S", METH("F", "view"))]
                                        + ([UseExt("package.S")] if pos else [])
                                        + [main(LET("s", C("S")), ES(M(N("s"), "F")))]))),
    "M03": ("check", lambda pos: prog(STRUCT("S", []),
                                      EXT("S", METH("F", "view"), METH("G" if pos else "F", "view")))),
    "P01": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")]),
                                      EXT("S", METH("F", "edit" if pos else "view",
                                                    [ASG(FLD(N("self"), "x"), I(2))])))),
    "H12": ("check", lambda pos: prog(STRUCT("S", []), EXT("S", METH("F", "view" if pos else "edit")),
                                      UseExt("package.S"), main(ES(M(C("S"), "F"))))),
    "H13": ("check", lambda pos: prog(STRUCT("S", []), EXT("S", METH("F", "view" if pos else "move")),
                                      UseExt("package.S"), main(ES(M(C("S"), "F"))))),
    "H12-edit": ("check", lambda pos: prog(STRUCT("S", []), EXT("S", METH("F", "edit")), UseExt("package.S"),
                                           main(*([VAR("s", C("S")), ES(M(N("s"), "F", prefix="edit"))] if pos
                                                  else [ES(M(C("S"), "F", prefix="edit"))])))),
    "H13-move": ("check", lambda pos: prog(STRUCT("S", []), EXT("S", METH("F", "move")), UseExt("package.S"),
                                           main(*([LET("s", C("S")), ES(M(N("s"), "F", prefix="move"))] if pos
                                                  else [ES(M(C("S"), "F", prefix="move"))])))),
    # ---- G6
    "N01": ("check", lambda pos: prog(ENUM("E", [("A", ["I64", "I64"])]),
                                      main(LET("e", V("E", "A", I(1), I(2))),
                                           MATCH(N("e"), ARM(PVar("E", "A", [PBind("x")] + ([PBind("y")] if pos else []))))))),
    "N02": ("check", lambda pos: prog(ENUM("E", [("A", []), ("B", [])]),
                                      main(MATCH(V("E", "A", null=True),
                                                 *([ARM(PVar("E", "A"))] + ([ARM(PVar("E", "B"))] if pos else [])))))),
    "H11": ("check", lambda pos: prog(ENUM("E", [("A", []), ("B", ["I64", "Bool"])]),
                                      main(MATCH(V("E", "A", null=True),
                                                 *([ARM(PVar("E", "A"))] +
                                                   ([ARM(PVar("E", "B", [PWild(), PWild()]))] if pos else [])))))),
    # ---- G7
    "X12": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")], derive=["Format"] if pos else []),
                                      main(LET("s", C("S", I(1))), ES(C("Print", N("s")))))),
    "X13": ("check", lambda pos: prog(STRUCT("S", [("x", "I64")], derive=["PartialEq"] if pos else []),
                                      main(LET("s", C("S", I(1))),
                                           AEQ(BIN(N("s"), "==", N("s")), B(True))))),
    "X14": ("check", lambda pos: prog(STRUCT("S", [("r", "I64" if pos else "MockResource")], derive=["Format"]))),
    "X15": ("check", lambda pos: prog(STRUCT("S", [], derive=["Format"] if pos else ["Format", "Format"]))),
    "X16": ("check", lambda pos: prog(STRUCT("F", [], derive=["Format"]) if pos
                                      else FN("F", [], [], derive=["Format"]))),
    "N10": ("check", lambda pos: prog(STRUCT("S", [], derive=["Format"] if pos else []),
                                      main(LET("s", C("S")), LET("t", S(N("s")))))),
    "N11": ("check", lambda pos: prog(STRUCT("S", [("r", "I64" if pos else "MockResource")], derive=["PartialEq"]))),
    "M08": ("check", lambda pos: prog(STRUCT("S", [], derive=["PartialEq" if pos else "Hash"]))),
    "O14": ("check", lambda pos: prog(STRUCT("S", [], derive=["PartialEq"] if pos else []),
                                      main(LET("a", C("Some", C("S"))), LET("x", BIN(N("a"), "!=", N("a")))))),
    "O15": ("check", lambda pos: prog(STRUCT("S", [], derive=["PartialEq"] if pos else []),
                                      main(LET("a", C("Ok", C("S")), t="Result<S,I64>"),
                                           LET("x", BIN(N("a"), "!=", N("a")))))),
    "O16": ("check", lambda pos: prog(STRUCT("S", [], derive=["PartialEq"] if pos else []),
                                      ENUM("E", [("Nada", []), ("Dato", ["S"])], derive=["PartialEq"]),
                                      main())),
    "H07": ("check", lambda pos: prog(STRUCT("S", [], derive=["Format"] if pos else []),
                                      main(LET("s", C("S")), LET("t", S("é\n", N("s")))), sep="\r\n")),
    "H08": ("check", lambda pos: prog(STRUCT("S", [], derive=["Format"] if pos else []),
                                      main(LET("s", C("S")), LET("t", S("é\"\\", N("s")))))),
    # ---- G8
    "X17": ("check", lambda pos: prog(FN("F", [], [], ret="()" if pos else None, with_="unrestricted", pub=True))),
    "X18": ("check", lambda pos: prog(FN("F", [], [], ret="()", with_="unrestricted" if pos else None, pub=True))),
    "X19": ("check", lambda pos: prog(STRUCT("S", [], pub=pos),
                                      FN("F", [("x", "view", "S")], [], ret="()", with_="unrestricted", pub=True))),
    "X20": ("check", lambda pos: prog(STRUCT("S", [], pub=pos), Use("package.S", "T", pub=True))),
    "M06": ("check", lambda pos: prog(STRUCT("Extra", []) if pos else RawDecl("mod extra;", "mod"))),
    "P03": ("check", lambda pos: prog(STRUCT("A", []), STRUCT("B", []), Use("package.A", "C" if pos else "B"))),
    "H10": ("check", lambda pos: prog(STRUCT("S", [], pub=pos), ENUM("E", [("A", ["S"])], pub=True))),
}
BUILD.update({k: ("check", v) for k, v in _op_rows("O01", OPS6, lambda pos, op: prog(
    main(LET("x", BIN(S("é"), op, S("z") if pos else I(1), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O02", OPS6, lambda pos, op: prog(
    main(LET("x", BIN(S("a") if pos else I(1), op, S("é"), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O03", OPS6[:4], lambda pos, op: prog(
    main(LET("x", BIN(S("a") if pos else B(True), op, S("b") if pos else B(False), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O05", ["==", "!="], lambda pos, op: prog(
    STRUCT("S", [], derive=["PartialEq"] if pos else []),
    main(LET("s", C("S")), LET("x", BIN(N("s"), op, N("s"), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O06", ["==", "!="], lambda pos, op: prog(
    STRUCT("S", [], derive=["PartialEq"] if pos else []),
    main(LET("a", ARR(), t="Array<S>"), LET("x", BIN(N("a"), op, N("a"), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O07", ["==", "!="], lambda pos, op: prog(
    STRUCT("A", [], derive=["PartialEq"]), STRUCT("B", [], derive=["PartialEq"]),
    main(LET("x", BIN(C("A"), op, C("A" if pos else "B"), sp=True))))).items()})
BUILD.update({k: ("check", v) for k, v in _op_rows("O08", OPS6[:4], lambda pos, op: prog(
    STRUCT("S", [], derive=["PartialEq"]),
    main(LET("s", C("S")), LET("x", BIN(N("s"), "==" if pos else op, N("s"), sp=True))))).items()})


def _tprog(entry, pos, use=False):
    pair = entry["pair"]
    arg = pair["positive_arg"] if pos else pair["negative_arg"]
    if pair["kind"] == "parse":
        text = arg[1:-1]
        callee = "parse.I64" if use else "std.parse.I64"
        decls = ([Use("std.parse")] if use else []) + [main(ES(C("Print", C(callee, S(text)))))]
        return prog(*decls)
    a, b = (int(x) for x in arg.split(","))
    return prog(main(ES(C("Print", M(S("é"), "SliceBytes", I(a), I(b))))))


def _results(entry, pos):
    pair = entry["pair"]
    return pair["positive_stdout"] if pos else entry["expected"]["stdout"]


def check_eq(rep: Report, name: str, got, want, detail=""):
    rep.check(name, got == want, f"{detail}{got!r} != {want!r}"[:300])


def _first(rep: Report, name: str, diags: list, want: dict):
    if len(diags) != 1:
        rep.check(name, False, f"{len(diags)} diagnostics: {[d['code'] + '/' + str(d['subcause']) for d in diags]}")
        return
    diff = diag_equal(diags[0], want)
    rep.check(name, not diff, ",".join(diff))


def run_row(rep: Report, ident: str, emap: dict):
    entry = emap[ident]
    kind, build = BUILD[ident][0], BUILD[ident][1]
    opts = BUILD[ident][2] if len(BUILD[ident]) > 2 else {}
    limits = opts.get("limits")
    mlim = {k: v for k, v in (limits or {}).items() if k != "profile_id"}
    profile = opts.get("profile")
    file = entry["file"]
    want = expected_diagnostic(entry)
    if kind == "check":
        src, diags = m.check_program(build(False), file, limits)
        rep.check(f"{ident}.source", src == entry["source"], repr(src)[:160])
        _first(rep, f"{ident}.diagnostic", diags, want)
        psrc, pdiags = m.check_program(build(True), file, limits)
        rep.check(f"{ident}.positive", psrc == entry["positive"]["source"] and not pdiags,
                  f"{psrc!r} {[d['code'] for d in pdiags]}"[:200])
    elif kind == "run":
        src, out, fault = m.run_program(build(False), file, mlim, profile)
        rep.check(f"{ident}.source", src == entry["source"], repr(src)[:160])
        _first(rep, f"{ident}.diagnostic", [fault] if fault else [], want)
        psrc, pout, pfault = m.run_program(build(True), file, mlim, profile)
        rep.check(f"{ident}.positive", psrc == entry["positive"]["source"] and pfault is None
                  and pout.fault is None, repr(psrc)[:160])


def run_t_rows(rep: Report, emap: dict, use: bool):
    for ident in sorted(i for i in emap if i.startswith("T") and i[1:].isdigit()):
        entry = emap[ident]
        pair = entry["pair"]
        if use and pair["kind"] != "parse":
            continue
        for pos in (False, True):
            if pos and pair["positive_arg"] is None:
                continue
            program = _tprog(entry, pos, use)
            src, out, fault = m.run_program(program, entry["file"])
            want_src = (pair["use_variant_positive" if pos else "use_variant"] if use
                        else (pair["positive_source"] if pos else entry["source"]))
            got = [utf8ref.from_bytes(b) for b in out.stdout]
            tag = f"{ident}.{'use.' if use else ''}{'pos' if pos else 'neg'}"
            rep.check(tag, src == want_src and got == _results(entry, pos) and fault is None
                      and not m.check_program(program, entry["file"])[1],
                      f"{src!r} {got!r}"[:200])


def run_result_rows(rep: Report, ids, emap: dict, builders: dict):
    for ident in ids:
        entry = emap[ident]
        program = builders[ident]()
        src, diags = m.check_program(program, entry["file"])
        rep.check(f"{ident}.source", src == entry["source"], repr(src)[:200])
        rep.check(f"{ident}.accepted", not diags, str([d["code"] for d in diags]))
        _, out, fault = m.run_program(program, entry["file"])
        e = entry["expected"]
        got_out = [utf8ref.from_bytes(b) for b in out.stdout]
        rep.check(f"{ident}.outcome", got_out == e["stdout"] and out.events == e["events"]
                  and fault is None and out.fault is None, f"{got_out!r} {out.events!r} {out.fault}"[:200])


def _A(n):
    def a01():
        return prog(main(LET("a", S("é🙂")), VAR("b", N("a")), ASG(N("b"), S("x")),
                         AEQ(M(N("a"), "ByteLen"), I(6)), AEQ(M(N("a"), "ScalarLen"), I(2)),
                         AEQ(N("b"), S("x"))), final_lf=True)

    def a02():
        return prog(main(LET("a", M(S("é"), "Scalars")), LET("b", M(S("é"), "Scalars")),
                         AEQ(N("a"), ARR(I(233))), AEQ(N("b"), ARR(I(101), I(769))), VAR("n", I(0)),
                         FOR("code", N("b"), [ASG(N("n"), I(1), "+=")]), ES(C("Print", N("n")))),
                    final_lf=True)

    def a03():
        return prog(Use("std.parse"), Use("std.text"),
                    main(LET("n", TRY(C("parse.I64", S("-9223372036854775808")))),
                         AEQ(C("text.FromI64", N("n")), S("-9223372036854775808")),
                         AEQ(C("parse.I64", S("+1")), C("Err", I(-9101))),
                         m.Ret(C("Ok", m.Raw("()", "()"))), ret="Result<(),I64>"),
                    breaks=(1,), final_lf=True)

    def a04():
        return prog(STRUCT("S", [("x", "Array<String>")], derive=["PartialEq"]),
                    main(LET("a", C("S", ARR(S("a")))), VAR("b", N("a")),
                         ES(C("Push", edit(FLD(N("b"), "x")), S("b"))),
                         AEQ(FLD(N("a"), "x"), ARR(S("a"))), AEQ(FLD(N("b"), "x"), ARR(S("a"), S("b"))),
                         AEQ(BIN(N("a"), "!=", N("b")), B(True)),
                         AEQ(C("Pop", edit(FLD(N("b"), "x"))), C("Some", S("b"))),
                         AEQ(BIN(N("a"), "==", N("b")), B(True)),
                         LET("c", C("JoinArrays", FLD(N("a"), "x"), ARR(S("é")))), VAR("n", I(0)),
                         FOR("text", N("c"), [ASG(N("n"), M(N("text"), "ByteLen"), "+=")]),
                         AEQ(N("n"), I(3))), breaks=(0,), final_lf=True)

    def a05():
        return prog(ENUM("E", [("Nada", []), ("Vivo", ["MockResource"])]),
                    main(LET("e", V("E", "Vivo", C("MockOpen", I(7)))),
                         MATCH(N("e"), ARM(PVar("E", "Nada")),
                               ARM(PVar("E", "Vivo", [PBind("r", "move")]), [ES(C("Inspect", N("r")))]),
                               move_=True)), breaks=(0,), final_lf=True)

    def a06():
        return prog(STRUCT("Par", [("a", "MockResource"), ("b", "MockResource")]),
                    main(LET("p", C("Par", lab("b", C("MockOpen", I(2))), lab("a", C("MockOpen", I(1))))),
                         ES(C("Drop", N("p")))), breaks=(0,), final_lf=True)

    def a07():
        return prog(FN("Siguiente", [("n", "edit", "I64")],
                       [ASG(N("n"), BIN(N("n"), "+", I(1))), m.Ret(N("n"))], ret="I64"),
                    main(VAR("n", I(0)),
                         ES(C("Print", S("{", N("n"), ":", C("Siguiente", edit(N("n"))), ":", N("n"), "}"))),
                         AEQ(N("n"), I(1))), breaks=(0,), final_lf=True)

    return {"A01": a01, "A02": a02, "A03": a03, "A04": a04, "A05": a05, "A06": a06, "A07": a07}[n]


def _a08(console: bool):
    inner = S("é")
    s_expr = S(inner, ":", IFE(B(True), I(1), I(2)))
    if console:
        return prog(Use("std.console"),
                    main(LET("s", s_expr), ES(TRY(C("console.Println", N("s")))),
                         m.Ret(C("Ok", m.Raw("()", "()"))), ret="Result<(),I64>"),
                    breaks=(0,), final_lf=True)
    return prog(main(LET("s", s_expr), ES(C("Print", N("s")))))


def _cmp6():
    ml = True
    lines = [AEQ(BIN(S("é"), "==", S("é")), B(True)), AEQ(BIN(S("é"), "!=", S("é")), B(True)),
             AEQ(BIN(S("é"), "<", S("🙂")), B(True)), AEQ(BIN(S("é"), "<=", S("é")), B(True)),
             AEQ(BIN(S("🙂"), ">", S("é")), B(True)), AEQ(BIN(S("é"), ">=", S("é")), B(True)),
             AEQ(BIN(S("a"), "<", S("aa")), B(True)), AEQ(BIN(S(""), "<", S("a")), B(True)),
             AEQ(BIN(S("a"), ">", S("aa")), B(False)),
             VAR("a", ARR(), t="Array<E>"),
             ES(C("Push", edit(N("a")), V("E", "Dato", C("S", S("é"))))),
             ES(C("Push", edit(N("a")), V("E", "Nada", null=True))),
             VAR("b", N("a")),
             AEQ(C("Pop", edit(N("b"))), C("Some", V("E", "Nada", null=True))),
             AEQ(BIN(N("a"), "!=", N("b")), B(True)),
             ES(C("Push", edit(N("b")), V("E", "Nada", null=True))),
             AEQ(BIN(N("a"), "==", N("b")), B(True)),
             LET("c", C("JoinArrays", N("a"), N("b"))),
             AEQ(C("Len", N("c")), I(4)),
             VAR("n", I(0)),
             FOR("item", N("c"), [MATCH(N("item"), ARM(PVar("E", "Nada")),
                                        ARM(PVar("E", "Dato", [PBind("s")]),
                                            [ASG(N("n"), M(FLD(N("s"), "text"), "ByteLen"), "+=")]))], ml=True),
             AEQ(N("n"), I(4))]
    return prog(STRUCT("S", [("text", "String")], derive=["PartialEq"]),
                ENUM("E", [("Nada", []), ("Dato", ["S"])], derive=["PartialEq"]),
                FN("Main", [], lines, ml=True), sep="\n", final_lf=True)


def _k04p():
    lines = [VAR("a", ARR(C("S", S("é")))), VAR("b", N("a")),
             ES(C("Push", edit(N("b")), C("S", S("x")))),
             AEQ(C("Len", N("a")), I(1)), AEQ(C("Len", N("b")), I(2)),
             VAR("bytes", I(0)),
             FOR("r", N("b"), [ASG(N("bytes"), M(FLD(N("r"), "text"), "ByteLen"), "+=")]),
             AEQ(N("bytes"), I(3)),
             VAR("es", ARR(V("E", "Nada", null=True))),
             ES(C("Push", edit(N("es")), V("E", "Dato", C("S", S("y"))))),
             LET("popped", C("Pop", edit(N("es")))),
             MATCH(N("popped"), ARM(PNone(), [AEQ(B(False), B(True))]), ARM(PSome(PBind("v")))),
             VAR("count", I(0)),
             FOR("e", C("JoinArrays", N("es"), N("es")), [ASG(N("count"), I(1), "+=")]),
             AEQ(N("count"), I(2))]
    return prog(STRUCT("S", [("text", "String")]), ENUM("E", [("Nada", []), ("Dato", ["S"])]),
                FN("Main", [], lines, ml=True), sep="\n", final_lf=True)


G_ROWS = {
    "G1": ["X04", "X23", "X24", "X25", "X26", "N09"],
    "G2": ["X22", "M05", "O04", "O10", "O11", "O13", "B01-text"]
          + [f"O01-{o}" for o in OPS6] + [f"O02-{o}" for o in OPS6] + [f"O03-{o}" for o in OPS6[:4]],
    "G3": ["X05", "X06", "X07", "X08", "X09", "X10", "X21", "X27", "M07", "N03", "N04", "N12", "P02",
           "P05", "P06", "H09", "O12", "B01-struct", "B01-enum"],
    "G4": ["X11", "M04", "N05", "N06", "N07", "N08", "O09"],
    "G5": ["M01", "M02", "M03", "P01", "H12", "H13", "H12-edit", "H13-move"],
    "G6": ["N01", "N02", "H11"],
    "G7": ["X12", "X13", "X14", "X15", "X16", "N10", "N11", "M08", "O14", "O15", "O16", "H07", "H08"]
          + [f"O05-{o}" for o in ["==", "!="]] + [f"O06-{o}" for o in ["==", "!="]]
          + [f"O07-{o}" for o in ["==", "!="]] + [f"O08-{o}" for o in OPS6[:4]],
    "G8": ["X17", "X18", "X19", "X20", "M06", "P03", "H10"],
}


def _rows_group(group):
    def runner(rep: Report):
        emap = entry_map()
        for ident in G_ROWS[group]:
            run_row(rep, ident, emap)
        extra = {
            "G2": lambda: (run_t_rows(rep, emap, False), run_result_rows(rep, ["A01", "A02"], emap,
                                                                      {"A01": _A("A01"), "A02": _A("A02")})),
            "G3": lambda: run_result_rows(rep, ["K04P"], emap, {"K04P": _k04p}),
            "G4": lambda: (run_result_rows(rep, ["A06"], emap, {"A06": _A("A06")}), _a06_failure(rep)),
            "G6": lambda: run_result_rows(rep, ["A05"], emap, {"A05": _A("A05")}),
            "G7": lambda: (run_result_rows(rep, ["A04", "A07", "CMP6"], emap,
                                           {"A04": _A("A04"), "A07": _A("A07"), "CMP6": _cmp6}),
                           _a08_checks(rep, emap)),
            "G8": lambda: (run_t_rows(rep, emap, True),
                           run_result_rows(rep, ["A03"], emap, {"A03": _A("A03")}), _p04(rep, emap)),
        }.get(group)
        if extra:
            extra()
    return runner


def _a06_failure(rep: Report):
    """K 11.1 A06: replacing the second argument by a failing call leaves only open:0:2 and close:0:2."""
    fallar = FN("Fallar", [], [ES(C("Fail")), m.Ret(C("MockOpen", I(1)))], ret="MockResource")
    program = prog(STRUCT("Par", [("a", "MockResource"), ("b", "MockResource")]), fallar,
                   main(LET("p", C("Par", lab("b", C("MockOpen", I(2))), lab("a", C("Fallar")))),
                        ES(C("Drop", N("p")))))
    _, out, _ = m.run_program(program, "A06f.n")
    line = [ln for ln in ktext().split("\n") if "El negativo din" in ln]
    cited = len(line) == 1 and "open:0:2" in line[0] and "close:0:2" in line[0] and "R0301" in line[0]
    rep.check("A06.failing_variant", cited and out.events == ["open:0:2", "close:0:2"]
              and out.fault is not None and out.fault.code == "R0301", str(out.events))


def _a08_checks(rep: Report, emap: dict):
    e = emap["A08"]
    src, diags = m.check_program(_a08(True), e["file"])
    rep.check("A08.source", src == e["source"], repr(src)[:200])
    _, out, _ = m.run_program(_a08(False), e["file"])
    got = [utf8ref.from_bytes(b) for b in out.stdout]
    rep.check("A08.interpolation_with_print", got == e["expected"]["stdout"] and not out.events, str(got))


def _p04(rep: Report, emap: dict):
    e = emap["P04"]
    use = prog(Use("package.S", "T"))
    for pos in (False, True):
        defs = prog(STRUCT("S", [], pub=pos))
        diags = m.check_import_access(defs, "P04-def.n", use, e["file"])
        if pos:
            rep.check("P04.positive", not diags and m.render(defs).source == e["positive_definition"])
        else:
            rep.check("P04.source", m.render(use).source == e["source"]
                      and m.render(defs).source == e["definition_source"])
            _first(rep, "P04.diagnostic", diags, expected_diagnostic(e))


def run_model_basics(rep: Report):
    """Pure functions of the text model against facts of K 3 and 10.3."""
    cases = [(b"", ("Err", -9101)), ("١".encode(), ("Err", -9101)), (b"0007", ("Ok", 7)),
             (b"-0", ("Ok", 0)), (b"-", ("Err", -9101)), (b"1x", ("Err", -9101)),
             (b"999999999999999999999x", ("Err", -9101)), (b"9223372036854775808", ("Err", -9102)),
             (b"-9223372036854775808", ("Ok", -(1 << 63))), (b"-9223372036854775809", ("Err", -9102)),
             (b"+1", ("Err", -9101)), (b" 1", ("Err", -9101))]
    bad = [c for c, want in cases if m.parse_i64(c) != want]
    rep.check("model.parse_i64", not bad, str(bad))
    sl = [((0, 1), ("Err", -9111)), ((0, 2), ("Ok", "é".encode())), ((0, 3), ("Err", -9110)),
          ((2, 0), ("Err", -9110)), ((-1, 0), ("Err", -9110)), ((0, 0), ("Ok", b"")),
          ((2, 2), ("Ok", b""))]
    bad = [a for a, want in sl if m.slice_bytes("é".encode(), *a) != want]
    rep.check("model.slice_bytes", not bad, str(bad))
    rep.check("model.slice_empty_at_eof", m.slice_bytes(b"", 0, 0) == ("Ok", b""))
    rep.check("model.slice_range_before_boundary", m.slice_bytes("\U0001F642".encode(), 0, 9) == ("Err", -9110))
    mn = -(1 << 63)
    rep.check("model.from_i64", [m.from_i64(n) for n in (0, 7, -7, mn, (1 << 63) - 1)]
              == [b"0", b"7", b"-7", b"-9223372036854775808", b"9223372036854775807"])
    rep.check("model.compare_prefix_and_signed_bytes",
              m.compare_op("<", b"a", b"aa") and m.compare_op(">", b"\xc3\xa9", b"z")
              and m.compare_op("<", "é".encode(), "\U0001F642".encode())
              and not m.compare_op("==", "é".encode(), "é".encode())
              and m.compare_op("!=", "é".encode(), "é".encode())
              and m.compare_op(">=", b"", b"") and m.compare_op("<=", b"", b""))
    rep.check("model.scalars_vs_bytelen", m.scalars_of("é".encode()) == [101, 769]
              and len("é".encode()) == 3 and utf8ref.scalar_len("\U0001F642".encode()) == 1)
    out_fmt = m.format_value(m.Res(True, m.Sv("é".encode()))) + b"|" + m.format_value(m.Opt(False)) \
        + b"|" + m.format_value([1, 2]) + b"|" + m.format_value(m.Res(True, ()))
    rep.check("model.format", out_fmt == "Ok(é)|None|[1, 2]|Ok(())".encode(), str(out_fmt))


def run_scc_variants(rep: Report):
    """K 4.2: one diagnostic per cyclic component, shortest cycle, rotated to the smallest offset."""
    two = prog(STRUCT("A", [("b", "B")]), STRUCT("B", [("a", "A")]))
    _, d = m.check_program(two, "c.n")
    rep.check("scc.two_cycle_primary_is_last_edge", len(d) == 1 and d[0]["primary_span"]["start"] == 25)
    sep = prog(STRUCT("A", [("x", "A")]), STRUCT("B", [("y", "B")]))
    _, d = m.check_program(sep, "c.n")
    rep.check("scc.one_per_component_ordered", len(d) == 2 and d[0]["primary_span"]["start"] < d[1]["primary_span"]["start"])
    dag = prog(STRUCT("A", [("l", "B"), ("r", "B")]), STRUCT("B", [("x", "I64")]))
    _, d = m.check_program(dag, "c.n")
    rep.check("scc.diamond_has_no_cycle", d == [])
    arr = prog(STRUCT("A", [("x", "Array<A>")]))
    _, d = m.check_program(arr, "c.n")
    rep.check("scc.array_cycle_is_deferred_not_layout",
              len(d) == 1 and d[0]["code"] == "NX-E0006" and d[0]["feature_id"] == "user_types")
    tri = prog(STRUCT("A", [("c", "C"), ("b", "B")]), STRUCT("B", [("a", "A")]), STRUCT("C", [("a", "A")]))
    _, d = m.check_program(tri, "c.n")
    rep.check("scc.shortest_cycle_one_diag", len(d) == 1 and d[0]["code"] == "NX-E1323")


def run_semantics_extras(rep: Report):
    """Positive/negative probes of rules the literal rows exercise only once."""
    s = prog(STRUCT("S", [("x", "I64"), ("y", "I64")]),
             main(LET("s", C("S", lab("y", I(2)), lab("x", I(1))))))
    _, d = m.check_program(s, "c.n")
    rep.check("extra.labels_reordered_accepted", d == [])
    s = prog(STRUCT("S", [("x", "I64")]), main(VAR("s", C("S", I(1))), ASG(FLD(N("s"), "x"), I(3), "+=")))
    _, d = m.check_program(s, "c.n")
    rep.check("extra.compound_assign_on_field_ok", d == [])
    e = prog(ENUM("E", [("A", []), ("B", [])]), main(MATCH(V("E", "A", null=True), ARM(PWild()))))
    _, d = m.check_program(e, "c.n")
    rep.check("extra.wildcard_covers_enum", d == [])
    r = prog(STRUCT("S", [("r", "MockResource")]), main(LET("s", C("S", C("MockOpen", I(1))))))
    out = m.run_program(r, "c.n")[1]
    rep.check("extra.scope_end_closes_resource", out.events == ["open:0:1", "close:0:1"], str(out.events))
    lim = m.run_program(prog(main(LET("a", S("ab")), LET("b", N("a")))), "c.n",
                        {"text_allocations": 1}, "p")[2]
    rep.check("extra.copy_of_place_costs_an_allocation",
              lim is not None and lim["data"]["observed"] == 2 and lim["data"]["runtime_budget_kind"] == "text_allocations")
    alias = m.run_program(prog(main(VAR("a", ARR(S("é"))), FOR("x", N("a"), [ES(M(N("x"), "ByteLen"))]),
                                    ES(C("Pop", edit(N("a")))))), "c.n", {"text_allocations": 1}, "p")
    rep.check("extra.for_alias_and_pop_do_not_copy", alias[2] is None and alias[1].fault is None)



# ------------------------------------------------------------------ O3: generators (V/N/L/B)
# Scale, minimums and corpus SHA (plan 4).  Expected values come from K through the model;
# the generator is also checked against the literal rows of K with scripted choices.

from . import generate as gen

SCALE = {f"G{i}": ((300, 300) if i < 2 else (500, 500)) for i in range(10)}
MIN_STRUCT = {f"G{i}": (20 if i < 2 else 30) for i in range(10)}
SEED_RANGE = range(0, 8)
G10_SEEDS = (0, 31)
EXTRA_LABELS = {
    "G5+": ("G5", ("K01", "K02", "K03", "K04", "K05", "K06", "K07", "K08")),
    "G6+": ("G6", ("K01", "K02", "K03", "K04", "K05", "K06", "K07", "K08")),
    "G7+": ("G7", ("K01", "K02", "K03", "K04", "K05", "K06", "K07", "K08")),
    "G8+": ("G8", ("K01", "K02", "K03", "K04", "K05", "K06", "K07", "K08", "K09")),
}


def gen_labels() -> list:
    """(label, phase, complete, valid, invalid, seeds, min structural)."""
    out = []
    for i in range(10):
        ph = f"G{i}"
        out.append((ph, ph, barrier.PHASES[ph][0], *SCALE[ph], SEED_RANGE, MIN_STRUCT[ph]))
    for label, (ph, comp) in EXTRA_LABELS.items():
        out.append((label, ph, comp, *SCALE[ph], (0,), MIN_STRUCT[ph]))
    out.append(("G10", "G10", barrier.PHASES["G10"][0], 1000, 1000, G10_SEEDS, 50))
    return out


def corpus_table() -> dict:
    """SHA-256 of the generated corpus per label/seed/class (the pinned table)."""
    table = {}
    for label, ph, comp, nv, ni, seeds, _ in gen_labels():
        for seed in seeds:
            recs, _, _ = gen.generate_seed(seed, ph, comp, nv, ni)
            for c in "VNLB":
                table[f"{label}|{seed}|{c}"] = gen.corpus_sha(recs, c)
    table.update(special_entries())
    return table


class FixedGen(gen.Gen):
    """Generator source with scripted names and values (reproduces the K rows)."""

    def __init__(self, **queues):
        super().__init__(None)
        self.q = {k: list(v) for k, v in queues.items()}

    def _pop(self, key):
        return self.q[key].pop(0)

    def var(self): return self._pop("var")
    def ty(self): return self._pop("ty")
    def fld(self): return self._pop("fld")
    def fn(self): return self._pop("fn")
    def vnt(self): return self._pop("vnt")
    def mth(self): return self._pop("mth")
    def i(self, lo=0, hi=20): return m.I(self._pop("i"))
    def text(self, escapes=False): return self._pop("text")
    def pad(self): return []
    def sep(self): return ""


class Scripted:
    """Random source that replays a list of choices (checked against the offered values)."""

    def __init__(self, values):
        self.q = list(values)

    def choice(self, seq):
        v = self.q.pop(0)
        assert v in seq, (v, seq)
        return v


ROW_CASES = [
    ("X04", "n_string_index", dict(var=["s"], text=["é"], i=[0])),
    ("X05", "n_unknown_field", dict(ty=["S"], fld=["x", "y"], var=["s"], i=[1])),
    ("X08", "n_missing_arg", dict(ty=["S"], fld=["x"], var=["s"], i=[1])),
    ("X09", "n_unknown_label", dict(ty=["S"], fld=["x", "z"], var=["s"], i=[1])),
    ("X10", "n_assign_let_field", dict(ty=["S"], fld=["x"], var=["s"], i=[1, 2])),
    ("X11", "n_copy_resource", dict(ty=["S"], fld=["r"], var=["s", "t"], i=[1])),
    ("X12", "n_print_no_format", dict(ty=["S"], fld=["x"], var=["s"], i=[1])),
    ("N01", "n_pattern_arity", dict(ty=["E"], vnt=["A"], var=["e", "x", "y"], i=[1, 2])),
    ("N02", "n_non_exhaustive", dict(ty=["E"], vnt=["A", "B"], var=["e"])),
    ("N03", "n_unknown_variant", dict(ty=["E"], vnt=["A", "B"], var=["e"])),
    ("N04", "n_nominal_mismatch", dict(ty=["A", "B"], var=["x"])),
    ("O10", "n_for_string", dict(var=["x"], text=["é"])),
    ("O12", "n_for_nominal", dict(ty=["S"], var=["x"])),
    ("P05", "n_non_callable", dict(ty=["Evento"], vnt=["Inicio"])),
    ("P06", "n_drop_field", dict(ty=["S"], fld=["campo"], var=["p"], i=[1])),
    ("M01", "n_missing_edit_prefix", dict(ty=["S"], mth=["F"], var=["s"])),
    ("M02", "n_no_use_extension", dict(ty=["S"], mth=["F"], var=["s"])),
    ("H12", "n_temp_edit", dict(ty=["S"], mth=["F"])),
    ("H13", "n_temp_move", dict(ty=["S"], mth=["F"])),
    ("H12-edit", "n_edit_non_place", dict(ty=["S"], mth=["F"], var=["s"])),
    ("B01-text", "n_b01_text", dict(var=["a", "x"], text=["x", "y"])),
    ("B01-struct", "n_b01_struct", dict(ty=["S"], fld=["x"], var=["a", "x"], i=[1, 2])),
]

L_ROW_CASES = [
    ("X01", "l_empty", ["", "", "", 0, "let", "s"]),
    ("H05", "l_empty", [" \t ", "é", "", 0, "let", "s"]),
    ("X02", "l_unescaped_close", ["", "", 0, "let", "s"]),
    ("X03", "l_unterminated", ["", 1, 0, "let", "s"]),
    ("L01", "l_incomplete_open", ["", 0, "let", "s"]),
    ("H06", "l_incomplete_expr", ["é", 1, 0, "let", "s"]),
    ("H04", "l_comment", ["é", 1, 0, "let", "s"]),
    ("H03", "l_raw_break", ["é", "\n", 1, 0, "let", "s"]),
    ("H01", "l_raw_break", ["é", "\r\n", 1, 0, "let", "s"]),
    ("H02", "l_raw_break", ["é", "\r", 1, 0, "let", "s"]),
]


def _fam(name):
    for f in gen.V_FAMS + gen.N_FAMS + gen.L_FAMS + gen.B_FAMS:
        if f.name == name:
            return f
    raise KeyError(name)


def _no_file(value):
    """Spans carry the case file name, which differs between a K row and a generated case."""
    if isinstance(value, dict):
        return {k: ("F" if k == "file" else _no_file(v)) for k, v in value.items()}
    if isinstance(value, list):
        return [_no_file(v) for v in value]
    return value


def _fields_match(got: dict, want: dict, names) -> list:
    return [n for n in names if _no_file(got.get(n)) != _no_file(want.get(n))]


def check_rows_reproduced(rep: Report):
    """The generator, driven by scripted choices, rebuilds each K row (source and diagnostic)."""
    emap = entry_map()
    bad = []
    for ident, fam_name, queues in ROW_CASES:
        entry = emap[ident]
        fam = _fam(fam_name)
        g = FixedGen(**queues)
        build = fam.build(g)
        program = m.Program(list(build(False)), sep="")
        rpt = {"cls": "N", "family": fam_name, "closed": sorted(barrier.ALL_K), "verbs": ["check", "run"],
               "fields": fam.fields, "program": gen.to_repr(program)}
        res = gen.expect(rpt)
        want = expected_diagnostic(entry)
        got = res["diags"][0] if res["diags"] else {}
        problems = []
        if res["source"] != entry["source"]:
            problems.append("source")
        diff = _fields_match(got, want, gen.fields_of(fam.fields))
        if diff:
            problems.append("fields " + ",".join(diff))
        pos = m.Program(list(fam.build(FixedGen(**queues))(True)), sep="")
        pres = gen.expect({"cls": "P", "family": fam_name, "closed": sorted(barrier.ALL_K),
                           "verbs": ["check", "run"], "fields": "", "program": gen.to_repr(pos)})
        if pres["status"] != "ok" or ("positive" in entry and pres["source"] != entry["positive"]["source"]):
            problems.append("positive")
        if problems:
            bad.append(f"{ident}: {'; '.join(problems)}")
    for ident, fam_name, script in L_ROW_CASES:
        entry = emap[ident]
        fam = _fam(fam_name)
        g = gen.Gen(Scripted(script))
        lit = fam.build(g)
        res = gen.expect({"cls": "L", "family": fam_name, "closed": sorted(barrier.ALL_K),
                          "verbs": ["check", "run"], "fields": fam.fields, "lit": lit})
        want = expected_diagnostic(entry)
        got = res["diags"][0] if res["diags"] else {}
        problems = []
        if res["source"] != entry["source"]:
            problems.append("source")
        diff = _fields_match(got, want, gen.fields_of(fam.fields))
        if diff:
            problems.append("fields " + ",".join(diff))
        if problems:
            bad.append(f"{ident}: {'; '.join(problems)}")
    rep.check("gen.rows_reproduced", not bad, "; ".join(bad[:5]))
    rep.check("gen.rows_count", len(ROW_CASES) + len(L_ROW_CASES) >= 32)


def run_gen(rep: Report):
    pinned = fio.load_json("f4_oracle_corpus_sha.json")
    got_table = {}
    discard_bad, short, fam_bad, mins_bad = [], [], [], []
    for label, ph, comp, nv, ni, seeds, min_s in gen_labels():
        fam_union = {}
        for seed in seeds:
            recs, st, fs = gen.generate_seed(seed, ph, comp, nv, ni)
            for c in "VNLB":
                got_table[f"{label}|{seed}|{c}"] = gen.corpus_sha(recs, c)
            for c in "VNLB":
                extra = set(st[c]["discards"]) - {"duplicate"}
                if extra:
                    discard_bad.append(f"{label}/{seed}/{c}: {sorted(extra)}")
            if st["V"]["distinct"] < nv:
                short.append(f"{label}/{seed}: V {st['V']['distinct']}<{nv}")
            inv = sum(st[c]["distinct"] for c in "NLB")
            if inv < ni:
                short.append(f"{label}/{seed}: invalid {inv}<{ni}")
            if len(st["V"]["structural"]) < min_s:
                mins_bad.append(f"{label}/{seed}: V structural {len(st['V']['structural'])}<{min_s}")
            inv_struct = set()
            for c in "NLB":
                inv_struct |= st[c]["structural"]
            if len(inv_struct) < min_s:
                mins_bad.append(f"{label}/{seed}: invalid structural {len(inv_struct)}<{min_s}")
            for name, f in fs.items():
                u = fam_union.setdefault(name, [set(), f["max_forms"]])
                u[0] |= f["structural"]
        for name, (forms, max_forms) in (fam_union.items() if len(seeds) > 1 else ()):
            need = 8 if max_forms is None else min(8, max_forms)
            if len(forms) < need:
                fam_bad.append(f"{label}/{name}: {len(forms)}<{need}")
    rep.check("gen.no_inconsistent_discards", not discard_bad, "; ".join(discard_bad[:4]))
    rep.check("gen.distinct_minimums", not short, "; ".join(short[:4]))
    rep.check("gen.structural_minimums", not mins_bad, "; ".join(mins_bad[:4]))
    rep.check("gen.family_structural_minimums", not fam_bad, "; ".join(fam_bad[:6]))
    got_table.update(special_entries())
    rep.check("gen.corpus_sha.scale", pinned["scale"] == {k: list(v) for k, v in SCALE.items()}
              and pinned["g10"] == [1000, 1000, 50], "scale differs from the pinned table")
    wrong = [k for k in got_table if pinned["entries"].get(k) != got_table[k]]
    extra_pin = [k for k in pinned["entries"] if k not in got_table]
    rep.check("gen.corpus_sha", not wrong and not extra_pin,
              f"python {pinned.get('python')}; differ: {wrong[:3]} extra: {extra_pin[:3]}")
    # JSON round trip of the representation and expected hashes (one seed, every label)
    bad_rt, bad_sha = [], []
    for label, ph, comp, nv, ni, seeds, _ in gen_labels():
        recs, _, _ = gen.generate_seed(1, ph, comp, 40, 40)
        for r in recs:
            back = json.loads(fio.dumps(r["repr"]))
            res = gen.expect(back)
            if utf8ref.text(res["source"]).hex() != r["source_hex"] or res["expected"] != r["expected"]:
                bad_rt.append(f"{label}/{r['family']}")
            if fio.sha256_text(fio.dumps(r["expected"])) != r["expected_sha256"]:
                bad_sha.append(f"{label}/{r['family']}")
    rep.check("gen.repr_roundtrip", not bad_rt, ", ".join(bad_rt[:4]))
    rep.check("gen.expected_sha256", not bad_sha, ", ".join(bad_sha[:4]))
    a = gen.generate_seed(2, "G3", barrier.PHASES["G3"][0], 120, 120)[0]
    b = gen.generate_seed(2, "G3", barrier.PHASES["G3"][0], 120, 120)[0]
    rep.check("gen.deterministic", gen.corpus_sha(a) == gen.corpus_sha(b) and len(a) > 200)
    rep.check("gen.seeds_differ", gen.corpus_sha(a) !=
              gen.corpus_sha(gen.generate_seed(3, "G3", barrier.PHASES["G3"][0], 120, 120)[0]))
    check_rows_reproduced(rep)
    check_gen_structure(rep)


def check_gen_structure(rep: Report):
    """Family tables, barrier gating, classes and reductions."""
    closed_g2 = barrier.PHASES["G2"][0]
    names = gen.family_table("G2", closed_g2)
    rep.check("gen.gating.G2", "n_unknown_field" not in names["N"] and "n_for_string" in names["N"]
              and "b_k03" not in names["B"] and "b_k04" in names["B"], str(names))
    rep.check("gen.gating.G0_no_N", gen.family_table("G0", barrier.PHASES["G0"][0])["N"] == [])
    rep.check("gen.gating.G9_no_B", gen.family_table("G9", barrier.PHASES["G9"][0])["B"] == [])
    bad_manifest = []
    for ph, comp in [("G3", ("K01", "K02", "K03")), ("G9", ("K01", "K02", "K03", "K04", "K05")),
                     ("G5", ("K01", "K02", "K03", "K04", "K05", "K06", "K09"))]:
        try:
            gen.generate_seed(0, ph, comp, 1, 1)
            bad_manifest.append(ph)
        except barrier.ManifestError:
            pass
    rep.check("gen.manifest_rejected", not bad_manifest, str(bad_manifest))
    recs, _, _ = gen.generate_seed(0, "G1", barrier.PHASES["G1"][0], 60, 60)
    fam_by_name = {f.name: f for f in gen.V_FAMS + gen.N_FAMS + gen.L_FAMS + gen.B_FAMS}
    bad = []
    for r in recs:
        f = fam_by_name[r["family"]]
        if r["role"] != "main":
            continue
        d = r["expected"].get("check", {}).get("diagnostics", [])
        if f.cls == "B" and d[0]["data"]["pending_scope"] != f.target:
            bad.append(r["family"])
        if f.cls == "L" and d[0]["subcause"] != f.sub:
            bad.append(r["family"])
        if f.cls == "N" and d[0]["code"] != f.code:
            bad.append(r["family"])
        if f.cls == "V" and d:
            bad.append(r["family"])
    rep.check("gen.class_contracts", not bad, ", ".join(bad[:4]))
    g0, _, _ = gen.generate_seed(0, "G0", barrier.PHASES["G0"][0], 20, 60)
    prim = {}
    for r in g0:
        if r["fclass"] == "B":
            prim.setdefault(r["family"], set()).add("primary_span" in r["expected"]["fields"])
    rep.check("gen.barrier_primary_only_for_literals",
              True in prim.get("b_k02", ()) and prim.get("b_k04") == {False} and prim.get("b_k09") == {False},
              str(prim))
    lpos = [r for r in g0 if r["fclass"] == "L" and r["role"] == "positive"]
    rep.check("gen.L_positive_hits_K02_barrier_at_G0",
              bool(lpos) and all(r["expected"]["check"]["diagnostics"][0]["data"]["pending_scope"] == "K02"
                                 for r in lpos))
    g3, _, _ = gen.generate_seed(0, "G3", barrier.PHASES["G3"][0], 20, 60)
    lpos3 = [r for r in g3 if r["fclass"] == "L" and r["role"] == "positive"]
    rep.check("gen.L_positive_accepted_with_K02",
              bool(lpos3) and all(r["expected"]["check"]["outcome"] == {"kind": "checked"}
                                  and r["expected"]["run"]["outcome"] == {"kind": "unit"} for r in lpos3))
    small, total = 0, 0
    for r in gen.generate_seed(4, "G9", barrier.ALL_K, 40, 80)[0]:
        if r["role"] != "main":
            continue
        cands = gen.reductions(r["repr"])
        total += len(cands)
        for c in cands[:6]:
            res = gen.expect(c)
            if len(res["source"]) < len(utf8ref.from_bytes(bytes.fromhex(r["source_hex"]))):
                small += 1
    rep.check("gen.reductions.produce_smaller", total > 100 and small > 100, f"{total} {small}")
    base = gen.generate_seed(4, "G9", barrier.ALL_K, 5, 5)[0][0]
    before = fio.dumps(base["repr"])
    gen.reductions(base["repr"])
    rep.check("gen.reductions.pure", fio.dumps(base["repr"]) == before)
    p1 = m.Program([main(LET("a", I(1)), ES(C("Print", BIN(N("a"), "+", I(2)))))])
    p2 = m.Program([main(LET("zz", I(99)), ES(C("Print", BIN(N("zz"), "+", I(7)))))])
    p3 = m.Program([main(LET("a", I(1)), ES(C("Print", BIN(N("a"), "-", I(2)))))])
    rep.check("gen.structural_form", gen.structural_program(p1) == gen.structural_program(p2)
              and gen.structural_program(p1) != gen.structural_program(p3))
    p4 = m.Program([main(LET("a", S("é")))])
    p5 = m.Program([main(LET("b", S("zzz")))])
    p6 = m.Program([main(LET("b", S("z", I(1))))])
    rep.check("gen.structural_string_content", gen.structural_program(p4) == gen.structural_program(p5)
              and gen.structural_program(p4) != gen.structural_program(p6))


def check_gen_subprocess(rep: Report):
    """Determinism across interpreter processes with different hash randomisation."""
    import subprocess
    outs = []
    for hs in ("1", "777"):
        env = dict(os.environ, PYTHONHASHSEED=hs, PYTHONUTF8="1")
        r = subprocess.run([sys.executable, "-B", "-m", "scripts.f4_oracle.cli", "corpus-sha",
                            "--label", "G3", "--seeds", "2:2"], capture_output=True, cwd=audit.ROOT, env=env,
                           timeout=120)
        outs.append((r.returncode, r.stdout))
    ref = gen.generate_seed(2, "G3", barrier.PHASES["G3"][0], *SCALE["G3"])[0]
    want = {c: gen.corpus_sha(ref, c) for c in "VNLB"}
    ok = all(rc == 0 for rc, _ in outs) and outs[0][1] == outs[1][1]
    lines = outs[0][1].decode().split()
    ok = ok and all(f"G3|2|{c}={want[c]}" in lines for c in "VNLB")
    rep.check("gen.deterministic_across_processes", ok, str(outs[0][0]) + str(lines[:2]))


# ------------------------------------------------------------------ O4: harness, mock executables

import contextlib
import shutil
import subprocess
import tempfile
from pathlib import Path

from . import batch as batch_mod, generic, minimize as minimize_mod, runner

REQUIRED_K = {"G1": "K02", "G2": "K03", "G3": "K04", "G4": "K05", "G5": "K06", "G6": "K07", "G7": "K08",
              "G8": "K09"}
ST_TMP = Path("C:/nexo-work/tmp")


def _record(seed, index, role, cls, family, source, verbs, expected, repr_=None, file="case.n", profile=None):
    return {"seed": seed, "index": index, "role": role, "cls": cls, "fclass": cls, "family": family,
            "phase": "G9", "file": file, "profile_id": profile or diagnostics.PROFILE,
            "source_hex": source.encode("utf-8").hex(), "verbs": verbs, "expected": expected,
            "expected_sha256": fio.sha256_text(fio.dumps(expected)), "repr": repr_}


def literal_records(phase: str, closed) -> list:
    """Records of the literal rows of K (full 21-field comparison) valid for ``phase``/``closed``."""
    emap = entry_map()
    closed = set(closed)
    ids = list(G0_LITERAL_IDS)
    for g in sorted(G_ROWS, key=lambda x: int(x[1:])):
        if int(g[1:]) <= int(phase[1:]) and REQUIRED_K[g] in closed:
            ids += G_ROWS[g]
    out = []
    for ident in ids:
        e = emap[ident]
        if e["expected"]["kind"] != "diagnostic" or not e.get("cli", True):
            continue
        if ident in BUILD and len(BUILD[ident]) > 2 and BUILD[ident][2].get("limits"):
            continue
        d = expected_diagnostic(e)
        fields = list(diagnostics.FIELDS) + (["message"] if "message" in d else [])
        has_main = "fn Main" in e["source"]
        verbs = ["check", "run"] if has_main else ["check"]
        exp = {"fields": fields}
        for v in verbs:
            exp[v] = gen.rejected_doc(diagnostics.PROFILE, [d], fields)
        out.append(_record(-1, len(out), "literal", "R", "row:" + ident, e["source"], verbs, exp, file=e["file"]))
        pos = e.get("positive")
        if pos:
            pexp = {"fields": []}
            for v in verbs:
                pexp[v] = (gen.checked_doc(diagnostics.PROFILE) if v == "check" else
                           gen._doc(diagnostics.PROFILE, 0, {"kind": "unit"}, [], [], [], [], []))
            out.append(_record(-1, len(out), "literal_positive", "R", "row:" + ident, pos["source"], verbs, pexp,
                               file=e["file"]))
    if int(phase[1:]) >= 9 and "K10" in closed:
        out += o5_literal_records(len(out))
    return out


# ---- mock harness plumbing

MOCK_NAME = "f4_oracle_mock_nexo.py"
DIAG_CORRUPTIONS = ["diag:" + f for f in diagnostics.FIELDS]
RESULT_CORRUPTIONS = ["exit", "outcome_kind", "outcome_value", "stdout", "events", "observations",
                      "observation_bytes", "reserved_observation_bytes"]
HARNESS_CORRUPTIONS = ["cardinality_extra", "cardinality_missing", "debug_release", "check_run",
                       "json_double", "json_absent", "timeout", "internal_error", "type_true", "type_string",
                       "type_float", "null_vs_absent", "arrays_reordered", "file_other", "panic_stderr",
                       "invalid_ir", "extra_diag_field", "extra_top_key", "stdout_extra_line"]
CORRUPTION_CATEGORY = {"timeout": "timeout_confirmed", "json_double": "non_json_output",
                       "json_absent": "non_json_output", "internal_error": "internal_error",
                       "panic_stderr": "panic_or_invalid_ir", "invalid_ir": "panic_or_invalid_ir"}
ROW_TARGETS = set(DIAG_CORRUPTIONS) | {"cardinality_extra", "cardinality_missing", "debug_release",
                                       "check_run", "null_vs_absent", "file_other", "extra_diag_field"}


class MockWorld:
    """Scratch directory, mock table and environment of one harness run."""

    def __init__(self):
        ST_TMP.mkdir(parents=True, exist_ok=True)
        self.dir = Path(tempfile.mkdtemp(prefix=f"f4-oracle-st-{os.getpid()}-", dir=str(ST_TMP)))
        self.mock = self.dir / "mock-nexo.py"
        self.mock.write_bytes(fio.read_fixture_bytes(MOCK_NAME))
        self.log = self.dir / "mock.log"

    def close(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def table(self, records, fuzz=False) -> Path:
        import hashlib
        sources = {}
        for r in records:
            if r.get("expected") is None:
                continue
            key = hashlib.sha256(bytes.fromhex(r["source_hex"])).hexdigest()
            for v in r["verbs"]:
                slot = key
                if r.get("env_hex") and v == "run":
                    slot = key + ":" + hashlib.sha256(bytes.fromhex(r["env_hex"])).hexdigest()
                sources.setdefault(slot, {})[v] = r["expected"][v]
        path = self.dir / "table.json"
        path.write_bytes(fio.dumps({"sources": sources, "fuzz": fuzz}).encode("utf-8"))
        return path

    def run(self, records, corrupt="", *, mode="compare", fuzz=False, timeout=10.0, workers=2, phase="G9",
            known=None, resume=False, output=None, sleep="3", binary=None, gate_info=None):
        corpus = self.dir / "corpus.jsonl"
        corpus.write_bytes(b"".join((fio.dumps(r) + "\n").encode("utf-8") for r in records))
        out = Path(output) if output else self.dir / "out.jsonl"
        env = {"F4_MOCK_TABLE": str(self.table(records, fuzz)), "F4_MOCK_CORRUPT": corrupt,
               "F4_MOCK_SLEEP": sleep, "F4_MOCK_LOG": str(self.log)}
        saved = {k: os.environ.get(k) for k in env}
        os.environ.update(env)
        binary = binary or self.mock
        try:
            if mode == "fuzz":
                code = runner.run_fuzz(corpus=corpus, binary_debug=binary, binary_release=binary,
                                       profile=diagnostics.PROFILE, output=out, resume=resume,
                                       workers=workers, timeout=timeout, tmp_root=ST_TMP)
            else:
                code = runner.run_compare(corpus=corpus, binary_debug=binary, binary_release=binary,
                                          profile=diagnostics.PROFILE, phase=phase,
                                          complete=list(barrier.ALL_K), output=out, resume=resume,
                                          known_path=known, workers=workers, timeout=timeout,
                                          tmp_root=ST_TMP, gate_info=gate_info)
        finally:
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
        return code, runner.read_jsonl(out)

    def calls(self) -> int:
        return len(self.log.read_text().splitlines()) if self.log.exists() else 0


def _findings(lines):
    return [x for x in lines if x.get("type") in ("finding",)]


def _summary(lines):
    return next(x for x in reversed(lines) if x.get("type") == "summary")


def _mock_records():
    """A K row (all 21 fields), a V case with two outputs and a generated N case."""
    row = next(r for r in literal_records("G3", barrier.PHASES["G3"][0]) if r["family"] == "row:X05"
               and r["role"] == "literal")
    recs, _, _ = gen.generate_seed(0, "G2", barrier.PHASES["G2"][0], 60, 60)
    v2 = next(r for r in recs if r["fclass"] == "V" and len(r["expected"]["run"]["stdout"]) >= 2)
    n1 = next(r for r in recs if r["fclass"] == "N" and r["role"] == "main")
    return row, v2, n1


def check_generic_units(rep: Report):
    nf = generic.normalize_files
    doc = {"a": [{"file": "/tmp/x/case.n"}, {"file": "other.n"}, {"file": "C:\\w\\case.n"}], "file": "case.n",
           "text": "case.n", "n": {"file": 3}}
    got = nf(doc, "case.n", "/tmp/x/case.n")
    rep.check("harness.normalize.only_file_field_of_the_case",
              got == {"a": [{"file": "case.n"}, {"file": "other.n"}, {"file": "case.n"}], "file": "case.n",
                      "text": "case.n", "n": {"file": 3}}, str(got))
    fd = generic._first_difference
    cases = [({"a": 1}, {"a": 1, "b": 2}, "$.b"), ({"a": 1, "b": 2}, {"a": 1}, "$.b"), ({"a": 1}, {"a": True}, "$.a"),
             ({"a": 1}, {"a": 1.0}, "$.a"), ({"a": 1}, {"a": "1"}, "$.a"), ({"a": None}, {}, "$.a"),
             ({"a": [1, 2]}, {"a": [2, 1]}, "$.a[0]"), ({"a": [1]}, {"a": [1, 2]}, "$.a"),
             ({"a": {"b": "é"}}, {"a": {"b": "é"}}, None), ({"a": [{"b": 1}]}, {"a": [{"b": 2}]}, "$.a[0].b"),
             (1, 1, None), ("1", 1, "$")]
    bad = [(e, a, w, fd(e, a)) for e, a, w in cases if fd(e, a) != w]
    rep.check("harness.first_difference.strict", not bad, str(bad[:2]))
    rep.check("harness.batches.copied_semantics",
              generic.make_batches("0:7", 3) == [[0, 1, 2], [3, 4, 5], [6, 7]] and
              list(generic.seed_range("3:5")) == [3, 4, 5] and list(generic.seed_range("4")) == [4],
              "make_batches/seed_range")
    try:
        generic.seed_range("5:2")
        ok = False
    except ValueError:
        ok = True
    rep.check("harness.seed_range.rejects_reversed", ok)
    pins = fio.load_json("f4_oracle_pins.json")
    rep.check("harness.pins.generic_symbols",
              set(pins["generic"]["symbols"]) == {"sha", "Invocation", "seed_range", "make_batches",
                                                  "normalize_files", "_first_difference", "invoke"}
              and {k for k, v in pins["generic"]["symbols"].items() if v["kind"] == "adapted"} ==
              {"normalize_files", "_first_difference", "invoke"})
    # audit detects a tampered pin, an adapted symbol without declared change and a stray symbol
    import copy
    tampered = copy.deepcopy(pins)
    tampered["generic"]["symbols"]["sha"]["ast_sha256"] = "0" * 64
    nochange = copy.deepcopy(pins)
    nochange["generic"]["symbols"]["invoke"]["changes"] = []
    stray = copy.deepcopy(pins)
    del stray["generic"]["symbols"]["make_batches"]
    rep.check("harness.audit.pin_hash_mismatch", bool(audit._generic_symbol_findings(tampered)))
    rep.check("harness.audit.adapted_needs_changes", bool(audit._generic_symbol_findings(nochange)))
    rep.check("harness.audit.symbol_set_exact", bool(audit._generic_symbol_findings(stray)))
    rep.check("harness.audit.generic_clean", audit._generic_symbol_findings(pins) == [])


def check_corruption_inventory(rep: Report):
    names = DIAG_CORRUPTIONS + RESULT_CORRUPTIONS + HARNESS_CORRUPTIONS
    rep.check("harness.corruptions.count", len(DIAG_CORRUPTIONS) == 21 and len(RESULT_CORRUPTIONS) == 8
              and len(HARNESS_CORRUPTIONS) >= 10 and len(names) >= 39, str(len(names)))
    r = subprocess.run([sys.executable, "-B", str(fio.fixture_path(MOCK_NAME)), "--list-corruptions"],
                       capture_output=True, timeout=30)
    listed = r.stdout.decode().split()
    rep.check("harness.corruptions.mock_implements_all", listed == names, f"{len(listed)} vs {len(names)}")


def check_mock_corruptions(rep: Report, world: MockWorld):
    row, v2, n1 = _mock_records()
    code, lines = world.run([row, v2, n1])
    s = _summary(lines)
    rep.check("harness.faithful.exit0", code == 0 and not _findings(lines) and s["records"] == 3
              and s["divergences"] == 0 and s["failures"] == 0, str(s))
    rep.check("harness.faithful.header", lines[0]["type"] == "header" and lines[0]["judge"] and lines[0]["adr"]
              and lines[0]["debug_sha256"] == lines[0]["release_sha256"])
    names = DIAG_CORRUPTIONS + RESULT_CORRUPTIONS + HARNESS_CORRUPTIONS
    for name in names:
        subset = [row] if name in ROW_TARGETS else [v2]
        code, lines = world.run(subset, name, timeout=1.0, sleep="4")
        cats = {f["category"] for f in _findings(lines)}
        want = CORRUPTION_CATEGORY.get(name, "divergence")
        ok = code == 1 and want in cats
        if name.startswith("diag:"):
            ok = ok and any(f"diagnostics[0].{name[5:]}" in f["path"] for f in _findings(lines))
        if name == "debug_release":
            ok = ok and "debug_release" in cats
        rep.check(f"harness.corruption.{name}", ok, f"exit={code} cats={sorted(cats)}")
    # the corruptions are real: a faithful run over each subset is clean (control)
    for label, subset in (("row", [row]), ("v2", [v2])):
        code, _ = world.run(subset)
        rep.check(f"harness.control.{label}", code == 0)


def check_mixed_family_divergences(rep: Report, world: MockWorld):
    """Divergences with code None (positive) and with a code (negative) in one family still summarise
    (runner sorted tuples with None and str and raised TypeError)."""
    recs, _, _ = gen.generate_seed(0, "G5", barrier.PHASES["G5"][0], 5, 40)
    fam = next(r["family"] for r in recs if r["role"] == "positive" and r["fclass"] == "N")
    pair = [r for r in recs if r["family"] == fam][:2]
    rep.check("harness.mixed_family_pair", len(pair) == 2 and {r["role"] for r in pair} == {"main", "positive"})
    for name in ("stdout_extra_line", "stdout", "outcome_kind"):
        try:
            code, lines = world.run(pair, name)
        except Exception as exc:
            rep.check(f"harness.mixed_family_summary.{name}", False, repr(exc))
            continue
        summ = [x for x in lines if x.get("type") == "summary"]
        rep.check(f"harness.mixed_family_summary.{name}", code == 1 and len(summ) == 1 and summ[0]["divergences"] >= 1,
                  str(code))
    keys = [x for x in lines if x.get("type") == "seed"]
    rep.check("harness.mixed_family_keys_total_order", bool(keys) and len(keys[0]["keys"]) >= 2)


def check_default_literals(rep: Report, world: MockWorld):
    """The literal rows of K are part of every default corpus (a gate without them would be a false PASS)."""
    from . import cli
    base = ["--profile", diagnostics.PROFILE, "--phase", "G10", "--complete", ",".join(barrier.ALL_K),
            "--seeds", "0:0", "--valid-per-seed", "20", "--invalid-per-seed", "20"]
    parser = cli.build_parser()
    closed = barrier.validate_manifest("G10", list(barrier.ALL_K))
    counts = {}
    for cmd, extra in (("generate", ["--output", "x"]), ("compare", ["--binary-debug", "d", "--binary-release", "r",
                                                                       "--output", "x"]),
                       ("gate", ["--binary-debug", "d", "--binary-release", "r", "--output", "x"])):
        args = parser.parse_args([cmd] + base + extra)
        counts[cmd] = sum(1 for r in cli.corpus_records(args, closed) if r["fclass"] == "R")
    args = parser.parse_args(["generate"] + base + ["--output", "x", "--no-literals"])
    off = sum(1 for r in cli.corpus_records(args, closed) if r["fclass"] == "R")
    rep.check("cli.literals_on_by_default", all(v > 150 for v in counts.values()) and off == 0, f"{counts} {off}")
    args = parser.parse_args(["generate"] + base + ["--output", "x"])
    rows = [r for r in cli.corpus_records(args, closed) if r["fclass"] == "R" and r["role"] == "literal"][:4]
    code, lines = world.run(rows, "diag:code")
    rep.check("cli.default_gate_fails_on_a_violated_literal_row", code == 1 and _findings(lines), str(code))


def check_harness_contract(rep: Report, world: MockWorld):
    row, v2, n1 = _mock_records()
    # a record without expected_sha256 or with a wrong one is rejected as a harness error
    for label, mutate in (("missing", lambda r: r.pop("expected_sha256")),
                          ("wrong", lambda r: r.__setitem__("expected_sha256", "0" * 64))):
        bad = dict(v2)
        mutate(bad)
        try:
            world.run([bad])
            ok = False
        except runner.HarnessError:
            ok = True
        rep.check(f"harness.record_{label}_expected_sha256_rejected", ok)
    other = dict(v2, profile_id="other-profile")
    other["expected_sha256"] = v2["expected_sha256"]
    try:
        world.run([other])
        ok = False
    except runner.HarnessError:
        ok = True
    rep.check("harness.profile_mismatch_rejected", ok)
    # the mock receives --mock-env only for run
    env_rec = dict(v2, env_hex="7b7d")
    world.log.write_text("")
    world.run([env_rec])
    log = world.log.read_text().splitlines()
    rep.check("harness.mock_env_only_on_run", log and all(("env" in ln) == ln.startswith("run") for ln in log),
              str(log[:4]))
    # known issues
    base = {"family": row["family"], "code": row["expected"]["run"]["diagnostics"][0]["code"],
            "path": "$.diagnostics[0].code", "max_phase": "G9", "cite": "K 10.1 X05", "finding": "H-test",
            "reviewed_by": "D"}
    kpath = world.dir / "known.json"

    def known(entries):
        kpath.write_bytes(fio.dumps({"version": 1, "entries": entries}).encode("utf-8"))
        return kpath

    code, lines = world.run([row], "diag:code", known=known([base]))
    s = _summary(lines)
    rep.check("harness.known_issue.counted_apart", code == 0 and s["known"] >= 1 and s["divergences"] == 0
              and all(f["category"] == "known" for f in _findings(lines)), str(s))
    code, lines = world.run([row], "diag:code", known=known([dict(base, max_phase="G1")]), phase="G9")
    rep.check("harness.known_issue.expires_after_max_phase", code == 1 and _summary(lines)["divergences"] >= 1)
    code, lines = world.run([row], "diag:code", known=known([dict(base, path="$.diagnostics[0].data")]))
    rep.check("harness.known_issue.needs_exact_match", code == 1)
    try:
        world.run([row], "", known=known([base]))
        ok = False
    except runner.HarnessError as exc:
        ok = "obsolete" in str(exc)
    rep.check("harness.known_issue.obsolete_exit2", ok)
    code, lines = world.run([v2], "diag:code", known=known([base]))
    rep.check("harness.known_issue.other_family_ignored", code == 0)
    for label, bad in (("reviewed_by_C", dict(base, reviewed_by="C")), ("no_cite", dict(base, cite="")),
                       ("bad_phase", dict(base, max_phase="G99"))):
        try:
            world.run([row], "diag:code", known=known([bad]))
            ok = False
        except runner.HarnessError:
            ok = True
        rep.check(f"harness.known_issue.rejects_{label}", ok)
    # resume by seed
    recs = []
    for seed in (0, 1, 2):
        r = dict(v2, seed=seed, index=0)
        recs.append(r)
    world.log.write_text("")
    out = world.dir / "resume.jsonl"
    code1, lines1 = world.run(recs, output=out)
    first_calls = world.calls()
    code2, lines2 = world.run(recs, resume=True, output=out)
    s2 = _summary(lines2)
    rep.check("harness.resume.skips_done_seeds", code1 == 0 and code2 == 0 and world.calls() == first_calls
              and s2["skipped_seeds"] == 3 and s2["records"] == 3, f"{first_calls} {world.calls()} {s2}")
    keep = [ln for ln in out.read_text(encoding="utf-8").split("\n") if ln.strip()]
    cut = [ln for ln in keep if json.loads(ln)["type"] != "summary"]
    last_seed = max(i for i, ln in enumerate(cut) if json.loads(ln)["type"] == "seed")
    out.write_text("\n".join(cut[:last_seed]) + "\n", encoding="utf-8")
    before = world.calls()
    code3, lines3 = world.run(recs, resume=True, output=out)
    rep.check("harness.resume.completes_interrupted_run", code3 == 0 and _summary(lines3)["records"] == 3
              and world.calls() - before == 4 and _summary(lines3)["skipped_seeds"] == 2,
              f"{world.calls() - before} {_summary(lines3)}")
    changed = world.dir / "mock-changed.py"
    changed.write_bytes(world.mock.read_bytes() + b"\n# changed\n")
    try:
        world.run(recs, resume=True, output=out, binary=changed)
        ok = False
    except runner.HarnessError as exc:
        ok = "resume refused" in str(exc)
    rep.check("harness.resume.refused_when_binary_changes", ok)
    rep.check("harness.cleanup.private_copies_removed",
              not (ST_TMP / f"f4-oracle-{os.getpid()}").exists())


def check_harness_fuzz(rep: Report, world: MockWorld):
    closed = barrier.PHASES["G1"][0]
    from . import cli
    recs = cli.fuzz_records(96, "G1", closed, 12, diagnostics.PROFILE, 10, 10)
    again = cli.fuzz_records(96, "G1", closed, 12, diagnostics.PROFILE, 10, 10)
    rep.check("harness.fuzz_corpus.deterministic", recs == again and len(recs) == 12
              and all(r["expected"] is None and r["expected_sha256"] and r["role"] == "fuzz" for r in recs))
    code, lines = world.run(recs, mode="fuzz", fuzz=True)
    rep.check("harness.fuzz.clean_exit0", code == 0 and _summary(lines)["records"] == 12, str(_summary(lines)))
    for name, cat in (("internal_error", "internal_error"), ("panic_stderr", "panic_or_invalid_ir"),
                      ("invalid_ir", "panic_or_invalid_ir"), ("timeout", "timeout_confirmed"),
                      ("debug_release", "debug_release")):
        code, lines = world.run(recs[:2], name, mode="fuzz", fuzz=True, timeout=1.0, sleep="4")
        rep.check(f"harness.fuzz.{name}", code == 1 and cat in {f["category"] for f in _findings(lines)},
                  f"exit={code}")


def check_harness_cli(rep: Report, world: MockWorld):
    from . import cli
    ns = ["--profile", diagnostics.PROFILE, "--phase", "G1", "--complete", "K01,K02", "--seeds", "0:1",
          "--valid-per-seed", "3", "--invalid-per-seed", "3"]
    gen_out = world.dir / "gen.jsonl"
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        c1 = cli.main(["generate"] + ns + ["--output", str(gen_out)])
        first = gen_out.read_bytes()
        c2 = cli.main(["generate"] + ns + ["--output", str(gen_out)])
    recs = runner.read_jsonl(gen_out)
    rep.check("cli.generate.deterministic_with_expected_first",
              c1 == 0 and c2 == 0 and first == gen_out.read_bytes() and recs
              and all("expected_sha256" in r and r["expected"] is not None for r in recs), buf.getvalue()[:80])
    errs = io.StringIO()
    with contextlib.redirect_stderr(errs):
        bad_manifest = cli.main(["generate", "--profile", diagnostics.PROFILE, "--phase", "G3", "--complete",
                                 "K01,K02,K03", "--seeds", "0", "--valid-per-seed", "1",
                                 "--invalid-per-seed", "1", "--output", str(gen_out)])
        bad_final = cli.main(["generate", "--final"] + ns + ["--output", str(gen_out)])
        bad_seeds = cli.main(["generate", "--profile", diagnostics.PROFILE, "--phase", "G1", "--complete",
                              "K01,K02", "--seeds", "5:2", "--valid-per-seed", "1", "--invalid-per-seed", "1",
                              "--output", str(gen_out)])
    rep.check("cli.exit2_on_harness_errors", bad_manifest == 2 and bad_final == 2 and bad_seeds == 2,
              f"{bad_manifest} {bad_final} {bad_seeds}")
    # compare / gate / fuzz through the CLI: faithful exit 0, corrupted exit != 0
    closed = barrier.PHASES["G1"][0]
    corpus = cli.corpus_records(cli.build_parser().parse_args(["generate"] + ns + ["--output", "x"]), closed)
    table_world = world
    table = table_world.table(corpus)
    common = ["--binary-debug", str(world.mock), "--binary-release", str(world.mock)] + ns + [
        "--timeout", "5", "--workers", "2"]
    results = {}
    env_backup = {k: os.environ.get(k) for k in ("F4_MOCK_TABLE", "F4_MOCK_CORRUPT", "F4_MOCK_LOG")}
    try:
        os.environ["F4_MOCK_TABLE"] = str(table)
        os.environ["F4_MOCK_LOG"] = str(world.log)
        for label, corrupt in (("faithful", ""), ("corrupt", "diag:code")):
            os.environ["F4_MOCK_CORRUPT"] = corrupt
            out = world.dir / f"cmp-{label}.jsonl"
            with contextlib.redirect_stdout(io.StringIO()):
                results["compare-" + label] = cli.main(["compare"] + common + ["--output", str(out)])
            out = world.dir / f"gate-{label}.jsonl"
            with contextlib.redirect_stdout(io.StringIO()):
                results["gate-" + label] = cli.cmd_gate(
                    cli.build_parser().parse_args(["gate"] + common + ["--output", str(out)]),
                    audit_fn=lambda: [], selftest_fn=lambda: 0)
        # gate does not compare when its selftest fails
        os.environ["F4_MOCK_CORRUPT"] = ""
        out = world.dir / "gate-failed.jsonl"
        with contextlib.redirect_stdout(io.StringIO()):
            results["gate-selftest-fails"] = cli.cmd_gate(
                cli.build_parser().parse_args(["gate"] + common + ["--output", str(out)]),
                audit_fn=lambda: [], selftest_fn=lambda: 1)
            results["gate-audit-fails"] = cli.cmd_gate(
                cli.build_parser().parse_args(["gate"] + common + ["--output", str(out)]),
                audit_fn=lambda: ["finding"], selftest_fn=lambda: 0)
    finally:
        for k, v in env_backup.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
    gate_lines = runner.read_jsonl(world.dir / "gate-faithful.jsonl")
    rep.check("cli.compare.exit_zero_when_faithful", results["compare-faithful"] == 0 and results["gate-faithful"] == 0,
              str(results))
    rep.check("cli.compare.exit_nonzero_on_divergence", results["compare-corrupt"] == 1, str(results))
    rep.check("cli.gate.exit_nonzero_on_divergence", results["gate-corrupt"] == 1, str(results))
    rep.check("cli.gate.records_audit_and_selftest", gate_lines[0]["type"] == "gate" and gate_lines[0]["audit_exit"] == 0
              and gate_lines[0]["selftest_exit"] == 0 and len(gate_lines[0]["selftest_log_sha256"]) == 64)
    rep.check("cli.gate.no_compare_when_selftest_or_audit_fails",
              results["gate-selftest-fails"] == 1 and results["gate-audit-fails"] == 1
              and not (world.dir / "gate-failed.jsonl").exists())
    # fuzz through the CLI
    fz = ["fuzz"] + common[:-4] + ["--fuzz-per-seed", "6", "--timeout", "5", "--workers", "2"]
    saved = {k: os.environ.get(k) for k in ("F4_MOCK_TABLE", "F4_MOCK_CORRUPT")}
    try:
        os.environ["F4_MOCK_TABLE"] = str(world.table([], fuzz=True))
        for label, corrupt, want in (("faithful", "", 0), ("internal_error", "internal_error", 1)):
            os.environ["F4_MOCK_CORRUPT"] = corrupt
            with contextlib.redirect_stdout(io.StringIO()):
                got = cli.main(fz + ["--output", str(world.dir / f"fz-{label}.jsonl")])
            rep.check(f"cli.fuzz.exit.{label}", got == want, f"{got}")
    finally:
        for k, v in saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v


@contextlib.contextmanager
def mock_env(world, table, corrupt):
    keys = {"F4_MOCK_TABLE": str(table), "F4_MOCK_CORRUPT": corrupt, "F4_MOCK_LOG": str(world.log)}
    saved = {k: os.environ.get(k) for k in keys}
    os.environ.update(keys)
    try:
        yield
    finally:
        for k, v in saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v


def _table_for_candidates(world, rec):
    """Mock table with the model's expectation of a record and of its (transitive) reductions."""
    import hashlib
    sources = {}
    todo = [rec["repr"]]
    seen = 0
    while todo and seen < 600:
        rp = todo.pop()
        seen += 1
        res = gen.expect(rp)
        if res["expected"] is None:
            continue
        key = hashlib.sha256(res["source"].encode("utf-8")).hexdigest()
        if key in sources:
            continue
        sources[key] = {v: res["expected"][v] for v in rec["verbs"] if v in res["expected"]}
        todo.extend(gen.reductions(rp))
    path = world.dir / "table-min.json"
    path.write_bytes(fio.dumps({"sources": sources, "fuzz": False}).encode("utf-8"))
    return path


def check_harness_minimize(rep: Report, world: MockWorld):
    recs, _, _ = gen.generate_seed(0, "G3", barrier.PHASES["G3"][0], 20, 80)
    cand = [r for r in recs if r["fclass"] == "N" and r["role"] == "main"
            and len(gen.from_repr(r["repr"]["program"]).decls[-1].body) >= 3]
    rec = cand[0]
    code, lines = world.run([rec], "diag:code")
    failure = next(f for f in _findings(lines) if f["binary"] == "debug" and f["verb"] == "check")
    table = _table_for_candidates(world, rec)
    kw = dict(binary_debug=world.mock, binary_release=world.mock, profile=diagnostics.PROFILE, tmp_root=ST_TMP)
    with mock_env(world, table, "diag:code"):
        res = minimize_mod.minimize(failure, **kw)
        same = False
        if res["status"] == "minimized":
            final = gen.expect(res["repr"])
            r2 = dict(rec, source_hex=res["source_hex"], expected=final["expected"],
                      expected_sha256=fio.sha256_text(fio.dumps(final["expected"])), repr=res["repr"])
            with runner.Session(world.mock, world.mock, tmp_root=ST_TMP) as s:
                same = any(f["path"] == failure["path"] and f["code"] == failure["code"]
                           and f["subcause"] == failure["subcause"] for f in s.check(r2))
        res0 = minimize_mod.minimize(failure, budget=0.0, **kw)
    rep.check("harness.minimize.reduces", res["status"] == "minimized" and res["steps"] >= 1
              and len(res["source_hex"]) < len(failure["source_hex"]), str((res["status"], res["steps"])))
    rep.check("harness.minimize.result_keeps_the_divergence", same, "no divergence after minimization")
    rep.check("harness.minimize.keeps_original_when_not_converged",
              res0["status"] == "not_converged" and res0["source_hex"] == failure["source_hex"], res0["status"])
    with mock_env(world, table, ""):
        resf = minimize_mod.minimize(failure, **kw)
    rep.check("harness.minimize.not_reproducible_keeps_original",
              resf["status"] == "not_reproducible" and resf["source_hex"] == failure["source_hex"], resf["status"])
    nr = minimize_mod.minimize(dict(failure, repr=None), **kw)
    rep.check("harness.minimize.no_repr", nr["status"] == "no_repr")


def check_harness_batch(rep: Report):
    base = dict(command="gate", binary_debug="C:/x/d", binary_release="C:/x/r", profile=diagnostics.PROFILE,
                phase="G3", complete="K01,K02,K03,K04", seeds="0:3", valid=5, invalid=5,
                output_dir="C:/nexo-work/tmp/f4-st-batch", width=2, session="s")
    plan = batch_mod.plan(**base)
    rep.check("harness.batch.plan", [p["label"] for p in plan] == ["00-01", "02-03"]
              and all("heavy-lock.py" in p["command"][2] and "run" in p["command"] and "--resume" in p["command"]
                      and p["command"][p["command"].index("--seeds") + 1] in ("0:1", "2:3") for p in plan))
    for label, gap, odir in (("gap_below_60", 30, "C:/nexo-work/tmp/x"), ("not_on_C", 75, "D:/nexo-work/x")):
        try:
            batch_mod.validate_scale(gap, Path(odir))
            ok = False
        except ValueError:
            ok = True
        rep.check(f"harness.batch.rejects_{label}", ok)
    d = Path(tempfile.mkdtemp(prefix="f4-st-batch-", dir=str(ST_TMP)))
    try:
        plan = batch_mod.plan(**dict(base, output_dir=d))
        calls, naps = [], []

        class R:
            returncode = 0

        def fake(cmd, **kw):
            calls.append(cmd)
            return R()

        code, report = batch_mod.run(plan, gap_seconds=75, runner=fake, sleep=lambda s: naps.append(s))
        rep.check("harness.batch.runs_with_gap", code == 0 and len(calls) == 2 and naps == [75], f"{calls} {naps}")
        plan[0]["output"].write_text(fio.dumps({"type": "summary", "exit": 0}) + "\n", encoding="utf-8")
        calls.clear()
        code, report = batch_mod.run(plan, gap_seconds=75, runner=fake, sleep=lambda s: None)
        rep.check("harness.batch.resume_skips_finished", len(calls) == 1 and report[0].get("skipped"), str(report))

        class Fail:
            returncode = 1

        calls.clear()
        plan[0]["output"].unlink()
        code, report = batch_mod.run(plan, gap_seconds=75, runner=lambda c, **k: (calls.append(c), Fail())[1],
                                     sleep=lambda s: None)
        rep.check("harness.batch.stops_on_failure", code == 1 and len(calls) == 1)
        code, report = batch_mod.run(plan, gap_seconds=75, dry_run=True)
        rep.check("harness.batch.dry_run", code == 0 and all("command" in r for r in report))
    finally:
        shutil.rmtree(d, ignore_errors=True)


def run_harness(rep: Report):
    """Tests of the harness against the fictitious executables (processes run outside the hook)."""
    guards_harness(rep)
    check_generic_units(rep)
    check_corruption_inventory(rep)
    world = MockWorld()
    try:
        check_mock_corruptions(rep, world)
        check_harness_contract(rep, world)
        check_harness_fuzz(rep, world)
        check_harness_cli(rep, world)
        check_mixed_family_divergences(rep, world)
        check_default_literals(rep, world)
        check_harness_minimize(rep, world)
        check_g9_harness(rep, world)
    finally:
        world.close()
    check_harness_batch(rep)
    rows = literal_records("G8", barrier.PHASES["G8"][0] + ("K07",))
    rep.check("harness.literal_records", len(rows) >= 60 and all(r["expected_sha256"] for r in rows), str(len(rows)))


# ------------------------------------------------------------------ O5: application, environment v2, quotas (G9, G10)

from . import app, charges

PROFILE = diagnostics.PROFILE
O5_ENV_IDS = ["E01", "E02", "E03", "E04", "E05", "E06"]
O5_APP_IDS = ["APP"] + sorted(app.VARIANT_NAMES) + ["APP-100"]
O5_Q_IDS = ["X24", "X25", "X26", "X27", "O13"]


def _row_doc(e: dict, profile=PROFILE) -> dict:
    """Result document of a K row of kind ``result`` (events and stdout in the observation order of K 6.1)."""
    x = e["expected"]
    order = app.interleave(x["events"], x["stdout"])
    outcome = {"kind": "ok"} if x["outcome"] == "ok" else {"kind": "err", "error": str(x["error"])}
    return gen._doc(profile, x["exit"], outcome, x["stdout"], x["events"], order, [], [])


def o5_literal_records(first: int) -> list:
    """Records of the rows of K 10.1, 10.6, 10.7 and 11 that need the environment or a test profile."""
    emap = entry_map()
    out = []

    def add(family, source, verbs, expected, file, profile=PROFILE, env_hex=None, role="literal"):
        rec = _record(-1, first + len(out), role, "R", family, source, verbs, expected, file=file,
                      profile=profile)
        if env_hex is not None:
            rec["env_hex"] = env_hex
        out.append(rec)

    for ident in O5_ENV_IDS:
        e = emap[ident]
        add("row:" + ident, e["source"], ["run"],
            {"fields": [], "run": app.controller_doc(e["expected"]["reason"])}, e["file"], env_hex=e["env_hex"])
        pos = gen._doc(PROFILE, 0, {"kind": "ok"}, [], [e["positive_event"]], [("event", e["positive_event"])],
                       [], [])
        add("row:" + ident, e["source"], ["check", "run"],
            {"fields": [], "check": gen.checked_doc(PROFILE), "run": pos}, e["file"],
            env_hex=e["env_positive_hex"], role="literal_positive")
    for ident in ["E07", "A08"] + O5_APP_IDS:
        e = emap[ident]
        add("row:" + ident, e["source"], ["check", "run"],
            {"fields": [], "check": gen.checked_doc(PROFILE), "run": _row_doc(e)}, e["file"],
            env_hex=e["env_hex"])
    for ident in O5_Q_IDS:
        e = emap[ident]
        profile = e["expected"]["profile_id"]
        d = expected_diagnostic(e)
        fields = list(diagnostics.FIELDS)
        has_main = "fn Main" in e["source"]
        if d["code"] == "R0002":
            exp = {"fields": fields, "check": gen.checked_doc(profile),
                   "run": gen._doc(profile, 1, {"kind": "simulator_fault"}, [], [], [], [d], fields)}
            pos_exp = {"fields": [], "check": gen.checked_doc(profile),
                       "run": gen._doc(profile, 0, {"kind": "unit"}, [], [], [], [], [])}
        else:
            exp = {"fields": fields, "check": gen.rejected_doc(profile, [d], fields)}
            pos_exp = {"fields": [], "check": gen.checked_doc(profile)}
        verbs = ["check", "run"] if has_main and d["code"] == "R0002" else ["check"]
        add("row:" + ident, e["source"], verbs, exp, e["file"], profile=profile)
        add("row:" + ident, e["positive"]["source"], verbs, pos_exp, e["file"], profile=profile,
            role="literal_positive")
    return out


def _stdout_len(lines) -> int:
    return sum(len(utf8ref.text(s)) + 1 for s in lines)


def check_app_rows(rep: Report):
    """K 11 rows: the model of the application reproduces every transcribed row."""
    emap = entry_map()
    k = ktext()
    for ident in O5_APP_IDS:
        e = emap[ident]
        x = e["expected"]
        if ident == "APP":
            env = app.base_env()
        elif ident == "APP-100":
            env = app.full_hundred_env()
        else:
            env = app.variant_env(app.VARIANT_NAMES[ident])
        rep.check(f"{ident}.env_bytes", app.env_bytes(env).hex() == e["env_hex"])
        run = app.run_app(app.parse_env(bytes.fromhex(e["env_hex"])))
        err = x.get("error")
        ok = (run.events == x["events"] and run.stdout == x["stdout"] and run.exit == x["exit"]
              and run.error == err and run.observation_bytes == x["observation_bytes"]
              and run.kind == x["outcome"])
        rep.check(f"{ident}.run", ok, f"{run.events[:3]} {run.kind} {run.error} {run.observation_bytes}")
        rep.check(f"{ident}.source_lines", e["source"].count("\n") == 81 and e["source"].endswith("}\n"))
        if ident in app.VARIANT_NAMES:
            frag = e["fragments"][2]["text"]
            rep.check(f"{ident}.row_matches_fragment",
                      all(("`%s`" % ev) in frag for ev in x["events"])
                      and re.search(r"; %d \|$" % x["observation_bytes"], frag) is not None)
    base = emap["APP"]["expected"]
    run = app.run_app(app.base_env())
    explicit = [(o[0], o[1]) for o in base["observations"]]
    rep.check("APP.observation_order", run.order == explicit, str(run.order[:4]))
    rep.check("APP.thirteen_observations", len(run.order) == 13)
    rep.check("APP.event_bytes_202_stdout_118",
              sum(9 + len(utf8ref.text(t)) for c, t in run.order if c == "event") == 202
              and _stdout_len(run.stdout) == 118)
    rep.check("APP.eleven_variants", len(app.VARIANT_NAMES) == 11)
    rep.check("APP-100.counts", len(emap["APP-100"]["expected"]["events"]) == 201
              and len(emap["APP-100"]["expected"]["stdout"]) == 101
              and "6.866" in k and "201 eventos y 101 salidas" in k)


APP_CASES = [
    # label, reads, writes, events, stdout, outcome, error
    ("boundary_discount", [("ok", "9"), ("ok", "10"), ("ok", "100"), ("ok", "101"), ("eof",)], [("ok",)] * 5,
     ["read_line:ok:1", "read_line:ok:2", "read_line:ok:3", "read_line:ok:3", "read_line:eof",
      "println:ok:22", "println:ok:23", "println:ok:25", "println:ok:25", "println:ok:51"],
     ["L1: 9 EUR => 1089 cent", "L2: 10 EUR => 1089 cent", "L3: 100 EUR => 10890 cent",
      "L4: 101 EUR => 12221 cent", "Registros: 4; total: 252 EUR y 89 cent; media: 6322"], "ok", None),
    ("zero_amount", [("ok", "0"), ("eof",)], [("ok",)] * 2,
     ["read_line:ok:1", "read_line:eof", "println:ok:19", "println:ok:45"],
     ["L1: 0 EUR => 0 cent", "Registros: 1; total: 0 EUR y 0 cent; media: 0"], "ok", None),
    ("minus_zero_and_zeros", [("ok", "-0"), ("ok", "007"), ("eof",)], [("ok",)] * 3,
     ["read_line:ok:2", "read_line:ok:3", "read_line:eof", "println:ok:20", "println:ok:23", "println:ok:48"],
     ["L1: -0 EUR => 0 cent", "L2: 007 EUR => 847 cent", "Registros: 2; total: 8 EUR y 47 cent; media: 423"],
     "ok", None),
    ("empty_lines_count", [("ok", ""), ("ok", ""), ("ok", "3"), ("eof",)], [("ok",)] * 2,
     ["read_line:ok:0", "read_line:ok:0", "read_line:ok:1", "read_line:eof", "println:ok:21", "println:ok:48"],
     ["L3: 3 EUR => 363 cent", "Registros: 1; total: 3 EUR y 63 cent; media: 363"], "ok", None),
    ("println_exhausted", [("ok", "1"), ("eof",)], [], ["read_line:ok:1", "read_line:eof", "println:err:-9000"],
     [], "err", -9000),
    ("eof_stops_reading", [("eof",), ("ok", "5")], [("ok",)],
     ["read_line:eof", "println:ok:45"], ["Registros: 0; total: 0 EUR y 0 cent; media: 0"], "ok", None),
    ("fin_is_case_sensitive", [("ok", "Fin"), ("ok", "fin")], [("ok",)], ["read_line:ok:3"], [], "err", -9101),
    ("non_ascii_rejected", [("ok", "é")], [("ok",)], ["read_line:ok:2"], [], "err", -9101),
    ("garbage_after_data", [("ok", "5"), ("ok", "1x")], [("ok",)], ["read_line:ok:1", "read_line:ok:2"], [],
     "err", -9101),
    ("negative_in_batch", [("ok", "5"), ("ok", "-1"), ("ok", "7")], [("ok",)],
     ["read_line:ok:1", "read_line:ok:2"], [], "err", 42),
    ("overflow_negative", [("ok", "-9223372036854775809")], [("ok",)], ["read_line:ok:20"], [], "err", -9102),
    ("max_i64_text_is_domain_error", [("ok", "9223372036854775807")], [("ok",)], ["read_line:ok:19"], [],
     "err", 42),
    ("read_error_midway", [("ok", "4"), ("err", 5), ("ok", "6")], [("ok",)], ["read_line:ok:1", "read_line:err:5"],
     [], "err", 5),
    ("write_error_in_summary", [("ok", "4"), ("eof",)], [("ok",), ("err", -2)],
     ["read_line:ok:1", "read_line:eof", "println:ok:21", "println:err:-2"],
     ["L1: 4 EUR => 484 cent"], "err", -2),
]


def check_app_cases(rep: Report):
    for label, reads, writes, events, stdout, outcome, error in APP_CASES:
        run = app.run_app(app.Env(reads, writes))
        nbytes = sum(9 + len(e) for e in events) + _stdout_len(stdout)
        ok = (run.events == events and run.stdout == stdout and run.kind == outcome and run.error == error
              and run.observation_bytes == nbytes and run.exit == (0 if outcome == "ok" else 1))
        rep.check(f"app.case.{label}", ok, f"{run.events} {run.stdout} {run.kind} {run.error}"[:240])
    hundred = app.Env([("ok", "10000")] * 100 + [("err", 70)], [("ok",)] * 101)
    run = app.run_app(hundred)
    rep.check("app.case.max_total_fits_i64",
              run.kind == "ok" and run.stdout[-1] == "Registros: 100; total: 1210000 EUR y 0 cent; media: 1210000"
              and len([e for e in run.events if e.startswith("read_line")]) == 100)
    ok_doc = app.run_app(app.base_env()).doc()
    err_doc = app.run_app(app.variant_env("read_err_70")).doc()
    rep.check("app.doc.shape", ok_doc["exit"] == 0 and ok_doc["outcome"] == {"kind": "ok"}
              and ok_doc["observation_bytes"] == 320 and ok_doc["reserved_observation_bytes"] == 0
              and err_doc["exit"] == 1 and err_doc["outcome"] == {"kind": "err", "error": "70"}
              and err_doc["observation_bytes"] == 25 and err_doc["diagnostics"] == [])


def check_env_model(rep: Report):
    emap = entry_map()
    bad = []
    for ident in O5_ENV_IDS:
        e = emap[ident]
        try:
            app.parse_env(bytes.fromhex(e["env_hex"]))
            bad.append(ident + ": accepted")
        except app.EnvError as exc:
            if exc.reason != e["expected"]["reason"]:
                bad.append(f"{ident}: {exc.reason}")
        pos = app.parse_env(bytes.fromhex(e["env_positive_hex"]))
        run = app.run_echo(pos)
        if (run.events != [e["positive_event"]] or run.observation_bytes != 23 or run.kind != "ok"
                or run.stdout != []):
            bad.append(ident + ": positive")
    rep.check("env.rows_E01_E06", not bad, "; ".join(bad))
    e6 = emap["E06"]
    rep.check("env.E06_bytes", bytes.fromhex(e6["env_hex"]).replace(b"\xc3\x28", b"\xc3\xa9")
              == bytes.fromhex(e6["env_positive_hex"]) and bytes.fromhex(e6["env_hex"]).count(b"\xc3\x28") == 1)
    e7 = app.run_echo(app.parse_env(bytes.fromhex(emap["E07"]["env_hex"])))
    rep.check("env.E07_control", e7.kind == "err" and e7.error == 70 and e7.events == ["read_line:err:70"]
              and e7.observation_bytes == 25 and e7.exit == 1)
    a08 = app.run_a08(app.parse_env(bytes.fromhex(emap["A08"]["env_hex"])))
    x = emap["A08"]["expected"]
    rep.check("env.A08", a08.stdout == x["stdout"] and a08.events == x["events"] and a08.exit == 0
              and a08.observation_bytes == 26)
    # the whole environment is validated, also after EOF (K 10.6)
    after_eof = (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],'
                 b'"mock_reply":[],"console_read_line":[{"ok":null},{"ok":7}],"console_println":[]}')
    try:
        app.parse_env(after_eof)
        rep.check("env.validated_after_eof", False, "accepted")
    except app.EnvError as exc:
        rep.check("env.validated_after_eof", exc.reason == "invalid_mock_schema")
    pairs = [
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[],"console_println":[]}', None),
        (b' { "console_println" : [ ] , "console_read_line" : [ { "ok" : "\\u00e9" } ] , "schema_version" : 2 ,'
         b' "mock_reply":[],"mock_request":[],"mock_write_i64":[],"mock_read_records":[] } \n', None),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"ok":"\\ud83d\\ude42\\/"}],"console_println":[]}', None),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"err":"-9223372036854775808"},{"err":"0"}],"console_println":[{"err":"9"}]}', None),
        (b'{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[],"console_println":[]}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[]}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[],"console_println":[],"x":1}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"err":"+1"}],"console_println":[]}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[],"console_println":[{"ok":"x"}]}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"ok":"a","err":"1"}],"console_println":[]}', "invalid_mock_schema"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"ok":"\\ude42"}],"console_println":[]}', "invalid_mock_json"),
        (b'{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
         b'"console_read_line":[{"ok":"\\ud83dx"}],"console_println":[]}', "invalid_mock_json"),
        (b'{"schema_version":2,', "invalid_mock_json"),
        (b"", "invalid_mock_json"),
        (b"[]", "invalid_mock_schema"),
        (b'{"schema_version":2} x', "invalid_mock_json"),
        (b'{"schema_version":02}', "invalid_mock_json"),
        (b"\xef\xbb\xbf{}", "invalid_mock_json"),
    ]
    bad = []
    for n, (data, want) in enumerate(pairs):
        try:
            app.parse_env(data)
            got = None
        except app.EnvError as exc:
            got = exc.reason
        if got != want:
            bad.append(f"{n}: {got} != {want}")
    rep.check("env.parser_pairs", not bad, "; ".join(bad))
    gaps = [b'{"a":1,"a":2}', b'{"a":"x\ty"}',
            b'{"a":' + b"[" * 17 + b"]" * 17 + b"}",
            b'{"schema_version":2,"mock_read_records":[1],"mock_write_i64":[],"mock_request":[],"mock_reply":[],'
            b'"console_read_line":[],"console_println":[]}']
    not_gap = []
    for n, data in enumerate(gaps):
        try:
            app.parse_env(data)
            not_gap.append(n)
        except app.ModelGap:
            pass
        except app.EnvError:
            not_gap.append(n)
    rep.check("env.unfixed_cases_are_gaps", not not_gap, str(not_gap))
    canon = [("0", True), ("-1", True), ("9223372036854775807", True), ("-9223372036854775808", True),
             ("9223372036854775808", False), ("-9223372036854775809", False), ("+1", False), ("007", False),
             ("-0", False), ("", False), ("-", False), ("1 ", False), ("1.0", False), (5, False)]
    rep.check("env.canonical_i64", all(app.canonical_i64(t) == w for t, w in canon),
              str([t for t, w in canon if app.canonical_i64(t) != w]))
    rng = random.Random("f4o-selftest-env")
    bad = []
    for _ in range(300):
        env, _shape = app.random_env(rng)
        data = app.env_bytes(env, rng)
        back = app.parse_env(data)
        if back.reads != env.reads or back.writes != env.writes:
            bad.append(data[:60])
    rep.check("env.roundtrip_with_layouts", not bad, str(bad[:2]))
    reasons = {}
    for name in app.SCHEMA_FAULTS + app.JSON_FAULTS:
        for k in range(12):
            r2 = random.Random("%s/%d" % (name, k))
            try:
                data = app.fault_env(r2, name)
                app.parse_env(data)
                got = "accepted"
            except app.EnvError as exc:
                got = exc.reason
            except app.ModelGap:
                got = "gap"
            if got != app.FAULT_REASON[name]:
                reasons.setdefault(name, set()).add(got)
    rep.check("env.fault_families_reasons", not reasons, str(reasons)[:300])


def _program_for(spec):
    """Model program of a straight-line recipe: ``spec`` is a list of ("lit", text) / ("copy", index)."""
    stmts, names = [], []
    for k, item in enumerate(spec):
        names.append("v%d" % k)
        stmts.append(LET(names[-1], S(item[1]) if item[0] == "lit" else N(names[item[1]])))
    return prog(main(*stmts)), stmts


def check_charges_against_model(rep: Report):
    """The closed-form accounting of ``charges`` agrees with the evaluator of ``model`` on random
    straight-line programs (K 8.2: the three quotas, their order and the ``observed`` values)."""
    rng = random.Random("f4o-selftest-charges")
    bad, faults = [], 0
    for n in range(300):
        spec, ops, sizes = [], [], []
        for k in range(rng.randint(1, 6)):
            if sizes and rng.random() < 0.4:
                j = rng.randrange(len(sizes))
                spec.append(("copy", j))
                ops.append(("copy", j, k))
                sizes.append(sizes[j])
            else:
                text = "".join(rng.choice(["a", "b", "é", "\U0001f642"]) for _ in range(rng.randint(0, 6)))
                spec.append(("lit", text))
                n_b = len(utf8ref.text(text))
                ops.append(("lit", n_b, k))
                sizes.append(n_b)
        limits = {"string_bytes": rng.randint(1, 14), "live_text_bytes": rng.randint(4, 40),
                  "text_allocations": rng.randint(1, 7)}
        st = charges.account(ops, dict(limits, nominal_members=16384))
        program, stmts = _program_for(spec)
        _, out, fault = m.run_program(program, "q.n", limits)
        got = out.fault
        if st.fault is None:
            if got is not None:
                bad.append(f"{n}: model faults")
            continue
        faults += 1
        node = stmts[st.fault.span].init
        if (got is None or got.data["runtime_budget_kind"] != st.fault.quota
                or got.data["limit"] != st.fault.limit or got.data["observed"] != st.fault.observed
                or got.key[0] is not node):
            bad.append(f"{n}: {st.fault.quota} {st.fault.observed} vs {got and got.data}")
    rep.check("charges.matches_model_random", not bad and faults > 60, f"{faults} {bad[:2]}")
    # the loop closed form agrees with the unrolled program
    bad = []
    for count in range(1, 6):
        for per in (1, 2, 3):
            for limit in (2, 3, 5, 7):
                ops = [("loop", count, [("lit", 0, k) for k in range(per)])]
                st = charges.account(ops, dict(charges.NORMAL, text_allocations=limit))
                spec = [("lit", "") for _ in range(count * per)]
                program, _ = _program_for(spec)
                _, out, _f = m.run_program(program, "q.n", {"text_allocations": limit})
                got_fault = out.fault
                # unrolled strings stay alive, but they are empty: only text_allocations can fire
                same = ((st.fault is None) == (got_fault is None)
                        and (st.fault is None or (got_fault.data["observed"] == st.fault.observed
                                                  and got_fault.data["runtime_budget_kind"] == st.fault.quota)))
                if not same:
                    bad.append(f"{count}/{per}/{limit}")
    rep.check("charges.loop_matches_unrolled_model", not bad, ", ".join(bad[:5]))
    # nominal_members against the checker of model (reduced and normal limits)
    bad = []
    for profile, (pid, limit) in [(p, v) for p, v in charges.REDUCED.items() if p == "nominal_members"]:
        for count in range(0, 6):
            rec = charges.r_members(random.Random(count), count, pid)
            fields = [("f%d" % i, "I64") for i in range(count)]
            _, diags = m.check_program(prog(STRUCT("S", fields)), charges.FILE,
                                       {"nominal_members": limit, "profile_id": pid})
            exp = rec.expected["check"]["diagnostics"]
            shape_ok = len(diags) == len(exp)
            if diags and shape_ok:
                for f in diagnostics.FIELDS:
                    a, b = diags[0].get(f), exp[0].get(f)
                    if f == "primary_span":
                        continue  # the name prefix differs (the recipe draws it); spans are checked below
                    shape_ok = shape_ok and a == b
            if not shape_ok:
                bad.append(count)
    rep.check("charges.members_matches_model", not bad, str(bad))


def check_q_recipes(rep: Report, final: bool):
    """Every recipe reaches its target quota first (K 8.3) and the K figures are met."""
    bad_first, bad_boundary, names = [], [], set()
    for seed in (0, 1, 7, 31):
        for r in charges.recipes(seed, final):
            lim = charges.limits_of(r.profile)
            occ = r.occupancy
            names.add(r.name)
            run_doc = r.expected["run"] if "run" in r.expected else r.expected["check"]
            diags = run_doc["diagnostics"]
            if r.target is None:
                if diags or any(occ[q] > lim[q] for q in lim):
                    bad_first.append(f"{r.name}/{seed}: passing recipe over a limit")
            else:
                d = diags[0]["data"]
                q = d.get("runtime_budget_kind") or d.get("budget_kind")
                if q != r.target or d["observed"] <= d["limit"] or d["limit"] != lim[q]:
                    bad_first.append(f"{r.name}/{seed}: {q}")
                others = [o for o in lim if o != q and occ[o] > lim[o]]
                if others:
                    bad_first.append(f"{r.name}/{seed}: {others} exceeded before the target")
                if q in ("text_allocations", "nominal_members") and d["observed"] != d["limit"] + 1:
                    bad_first.append(f"{r.name}/{seed}: observed {d['observed']} is not limit+1")
    rep.check("Q.target_quota_first" + (".final" if final else ""), not bad_first, "; ".join(bad_first[:4]))
    if final:
        long_ones = [r for r in charges.recipes(0, True) if r.long]
        got = {(r.name, r.target) for r in long_ones}
        rep.check("Q.final_recipes_of_K83",
                  {("string_literal", "string_bytes"), ("live_chain", "live_text_bytes"),
                   ("alloc_loop", "text_allocations"), ("members", "nominal_members")} <= got, str(got))
        five = charges.r_live_chain(random.Random(1), 60000, 4)
        d = five.expected["run"]["diagnostics"][0]["data"]
        rep.check("Q.five_strings_of_60000", d["observed"] == 300000 and d["limit"] == 262144
                  and five.occupancy["live_text_bytes"] == 240000 and d["runtime_budget_kind"] == "live_text_bytes")
        for count, observed in ((4097, 4097),):
            r = charges.r_alloc_loop(random.Random(2), count, 1)
            dd = r.expected["run"]["diagnostics"][0]["data"]
            rep.check("Q.alloc_4097", dd["observed"] == observed and dd["limit"] == 4096)
        big = charges.r_string_literal(random.Random(3), 65537)
        sp = big.expected["run"]["diagnostics"][0]["primary_span"]
        rep.check("Q.literal_65537", sp["start"] == 16 and sp["end"] == 16 + 65537 + 2
                  and big.expected["run"]["diagnostics"][0]["data"]["observed"] == 65537)
        mem = charges.r_members(random.Random(4), 16385)
        dm = mem.expected["check"]["diagnostics"][0]
        rep.check("Q.members_16385", dm["data"]["observed"] == 16385 and dm["data"]["limit"] == 16384
                  and dm["code"] == "NX-E0003" and dm["stage"] == "types" and dm["operation"] == "analysis.limit")
    else:
        rep.check("Q.nonfinal_excludes_long", all(not r.long for r in charges.recipes(0, False)))
        rep.check("Q.recipe_names", {"string_literal", "live_chain", "alloc_loop", "members", "string_8",
                                     "live_16", "alloc_2"} <= names, str(names))
        passing = {r.name: r for r in charges.recipes(1, False) if r.target is None}
        boundary = {"string_literal": ("string_bytes", 65536), "live_chain": ("live_text_bytes", 262144),
                    "alloc_loop": ("text_allocations", 4096), "members": ("nominal_members", 16384)}
        for name, (q, v) in boundary.items():
            reached = [r for r in charges.recipes(1, False) if r.name == name and r.occupancy[q] == v
                       and r.profile == charges.PROFILE]
            if not reached:
                bad_boundary.append(name)
        rep.check("Q.boundaries_reached_exactly", not bad_boundary, str(bad_boundary))
        rep.check("Q.passing_have_no_diagnostics", bool(passing))


def check_special_corpus(rep: Report):
    """Classes A, E and Q: counts, minimums, determinism, record validity and family coverage."""
    validate_record = runner.validate_record
    bad_valid, bad_count, bad_min = [], [], []
    e_fams, q_names, q_src = set(), set(), set()
    for label, ph, comp, nv, ni, seeds, min_s in gen_labels():
        if label not in ("G9", "G10"):
            continue
        for seed in seeds:
            recs = list(_special(seed, ph, comp, nv, ni, False))
            for r in recs:
                try:
                    validate_record(r)
                except runner.HarnessError as exc:
                    bad_valid.append(str(exc)[:80])
            a = [r for r in recs if r["fclass"] == "A"]
            e = [r for r in recs if r["fclass"] == "E"]
            q = [r for r in recs if r["fclass"] == "Q"]
            if len(a) != nv // 10 or len({r["distinct"] for r in a}) != len(a):
                bad_count.append(f"{label}/{seed}: A {len(a)}")
            if len(e) != ni // 25 or len({r["distinct"] for r in e}) != len(e):
                bad_count.append(f"{label}/{seed}: E {len(e)}")
            need = 30 if label == "G9" else 50
            if len({r["structural"] for r in a}) < need:
                bad_min.append(f"{label}/{seed}: A structural {len({r['structural'] for r in a})}<{need}")
            e_fams |= {r["family"] for r in e}
            q_names |= {r["family"] for r in q}
            if label == "G9":
                q_src |= {r["distinct"] for r in q}
    rep.check("corpus.AEQ.records_valid", not bad_valid, "; ".join(bad_valid[:3]))
    rep.check("corpus.AEQ.counts_and_distinct", not bad_count, "; ".join(bad_count[:3]))
    rep.check("corpus.A.structural_minimums", not bad_min, "; ".join(bad_min[:3]))
    want = {"env_" + n for n in app.SCHEMA_FAULTS + app.JSON_FAULTS} | {"env_valid"}
    rep.check("corpus.E.fault_families_covered", want <= e_fams, str(sorted(want - e_fams))[:200])
    rep.check("corpus.Q.variants_at_least_120", len(q_src) >= 120, str(len(q_src)))
    rep.check("corpus.Q.families", {"q_string_literal", "q_live_chain", "q_alloc_loop", "q_members"} <= q_names,
              str(sorted(q_names)))
    a1 = app.special_records(3, "G9", barrier.ALL_K, 500, 500, False)
    a2 = app.special_records(3, "G9", barrier.ALL_K, 500, 500, False)
    rep.check("corpus.AEQ.deterministic", gen.corpus_sha(a1) == gen.corpus_sha(a2) and len(a1) > 100
              and gen.corpus_sha(a1) != gen.corpus_sha(app.special_records(4, "G9", barrier.ALL_K, 500, 500, False)))
    rep.check("corpus.AEQ.needs_K10", app.special_records(0, "G8", barrier.PHASES["G8"][0], 500, 500, False) == []
              and app.special_records(0, "G9", barrier.PHASES["G9"][0], 50, 50, False) != [])
    # expected values re-derived from the environment bytes (serialisation and parsing are inverse)
    bad = []
    for r in a1:
        if r["fclass"] == "A":
            again = app.run_app(app.parse_env(bytes.fromhex(r["env_hex"])))
            if again.doc() != r["expected"]["run"]:
                bad.append(r["index"])
        elif r["fclass"] == "E" and r["family"] == "env_valid":
            again = app.run_echo(app.parse_env(bytes.fromhex(r["env_hex"])))
            if again.doc() != r["expected"]["run"]:
                bad.append(r["index"])
        elif r["fclass"] == "E":
            try:
                app.parse_env(bytes.fromhex(r["env_hex"]))
                bad.append(r["index"])
            except app.EnvError as exc:
                if r["expected"]["run"] != app.controller_doc(exc.reason):
                    bad.append(r["index"])
    rep.check("corpus.AE.expected_rederived_from_env", not bad, str(bad[:4]))
    outcomes = {r["expected"]["run"]["outcome"]["kind"] for r in a1 if r["fclass"] == "A"}
    rep.check("corpus.A.outcome_variety", outcomes == {"ok", "err"}, str(outcomes))


@functools.lru_cache(maxsize=None)
def _special(seed, ph, comp, nv, ni, final) -> tuple:
    return tuple(app.special_records(seed, ph, tuple(comp), nv, ni, final))


def special_entries() -> dict:
    """Pinned SHA-256 of classes A, E and Q (labels G9, G10 and, final recipes only, G10F)."""
    table = {}
    for label, ph, comp, nv, ni, seeds, _ in gen_labels():
        if label not in ("G9", "G10"):
            continue
        for seed in seeds:
            recs = _special(seed, ph, comp, nv, ni, False)
            for c in "AEQ":
                table[f"{label}|{seed}|{c}"] = gen.corpus_sha(recs, c)
    for seed in G10_SEEDS:
        recs = _special(seed, "G10", barrier.PHASES["G10"][0], 1000, 1000, True)
        table[f"G10F|{seed}|Q"] = gen.corpus_sha(recs, "Q")
    return table


def run_g9(rep: Report):
    guards_charges(rep)
    check_app_rows(rep)
    check_app_cases(rep)
    check_env_model(rep)
    check_charges_against_model(rep)
    check_q_recipes(rep, False)
    check_o5_literal_records(rep)


def run_g10(rep: Report):
    check_q_recipes(rep, True)
    check_special_corpus(rep)
    matrix = matrix_report()
    rep.check("G10.matrix_C01_C11", sorted(matrix) == ["C%02d" % i for i in range(1, 12)]
              and all(v["status"] in ("PASA", "NO EVALUADO", "PARCIAL") and v["note"] for v in matrix.values()))


def check_o5_literal_records(rep: Report):
    validate_record = runner.validate_record
    recs = o5_literal_records(0)
    bad = []
    for r in recs:
        try:
            validate_record(r)
        except runner.HarnessError as exc:
            bad.append(str(exc)[:80])
    rep.check("G9.literal_records_valid", not bad and len(recs) == 6 * 2 + 2 + 13 + 5 * 2, f"{len(recs)} {bad[:2]}")
    fams = {r["family"] for r in recs}
    rep.check("G9.literal_records_rows",
              {"row:" + i for i in O5_ENV_IDS + O5_APP_IDS + O5_Q_IDS + ["E07", "A08"]} <= fams)
    test_profiles = {r["profile_id"] for r in recs if r["profile_id"] != PROFILE}
    rep.check("G9.literal_records_test_profiles", test_profiles == {
        "f0-0004-b-test-string-8-v1", "f0-0004-b-test-text-live-16-v1", "f0-0004-b-test-text-alloc-2-v1",
        "f0-0004-b-test-members-2-v1"}, str(test_profiles))
    ctl = [r for r in recs if r["family"] in ("row:E01", "row:E05") and r["role"] == "literal"]
    rep.check("G9.controller_expected_partial",
              all(r["expected"]["run"] == {"exit": 2, "outcome": {"kind": "driver_error", "reason": r2}}
                  for r, r2 in zip(ctl, ("invalid_mock_schema", "invalid_mock_json"))))
    # every Q row: the closed form of charges reaches the same diagnostic as the row (X24-X26, O13)
    emap = entry_map()
    for ident, (profile, ops_spec) in {
            "X24": ("f0-0004-b-test-string-8-v1", [("lit", 9)]),
            "X25": ("f0-0004-b-test-text-live-16-v1", [("lit", 8), ("lit", 8), ("lit", 1)]),
            "X26": ("f0-0004-b-test-text-alloc-2-v1", [("lit", 0), ("lit", 0), ("lit", 0)])}.items():
        ops = [(k, n, i) for i, (k, n) in enumerate(ops_spec)]
        st = charges.account(ops, charges.limits_of(profile))
        want = emap[ident]["expected"]["data"]
        rep.check(f"Q.{ident}.closed_form", st.fault is not None and st.fault.quota == want["runtime_budget_kind"]
                  and st.fault.limit == want["limit"] and st.fault.observed == want["observed"])


# ---- harness over the O5 classes with the fictitious executables

def check_g9_harness(rep: Report, world: "MockWorld"):
    recs = (app.app_records(0, 6) + app.env_records(0, 8) + charges.q_records(0, False))
    sample = [r for r in recs if r["fclass"] == "A"][:4]
    sample += [r for r in recs if r["fclass"] == "E" and r["family"] == "env_valid"][:1]
    sample += [r for r in recs if r["fclass"] == "E" and r["family"] != "env_valid"][:2]
    sample += [r for r in recs if r["fclass"] == "Q" and r["profile_id"] != PROFILE][:3]
    sample += [r for r in recs if r["fclass"] == "Q" and r["profile_id"] == PROFILE and
               r["expected"]["fields"]][:1]
    sample += [r for r in o5_literal_records(0) if r["family"] in ("row:E05", "row:APP-V05", "row:X25")]
    code, lines = world.run(sample)
    summary = _summary(lines)
    rep.check("G9.harness.faithful_exit0", code == 0 and not _findings(lines) and summary["records"] == len(sample),
              str(code) + str(_findings(lines)[:1])[:200])
    err = next(r for r in sample if r["fclass"] == "A" and r["expected"]["run"]["outcome"]["kind"] == "err")
    ok = next(r for r in sample if r["fclass"] == "A" and r["expected"]["run"]["outcome"]["kind"] == "ok")
    ctl = next(r for r in sample if r["fclass"] == "E" and r["family"] != "env_valid")
    for name, target, must in [("exit", ok, True), ("outcome_kind", ok, True), ("outcome_value", err, True),
                               ("stdout", ok, True), ("events", ok, True), ("observations", ok, True),
                               ("observation_bytes", ok, True), ("reserved_observation_bytes", ok, True),
                               ("debug_release", ok, True), ("check_run", ok, True),
                               ("exit", ctl, True), ("outcome_kind", ctl, True), ("outcome_value", ctl, True),
                               ("exit", err, True)]:
        code, lines = world.run([target], corrupt=name)
        rep.check(f"G9.harness.corruption.{name}.{target['fclass']}", (code != 0) == must, f"exit {code}")
    # the same source with two environments is answered per environment by the mock
    two = [ok, err]
    code, lines = world.run(two)
    rep.check("G9.harness.same_source_two_envs", code == 0 and ok["source_hex"] == err["source_hex"], str(code))
    # an expected record without its hash is refused
    broken = dict(ok)
    broken.pop("expected_sha256")
    try:
        code, _ = world.run([broken])
    except runner.HarnessError:
        code = 2
    rep.check("G9.harness.record_without_hash_refused", code == 2)
    # a reduced profile other than the K 8.3 prefix is refused
    foreign = dict(next(r for r in sample if r["fclass"] == "Q" and r["profile_id"] != PROFILE),
                   profile_id="f0-0002-v2")
    try:
        code, _ = world.run([foreign])
    except runner.HarnessError:
        code = 2
    rep.check("G9.harness.foreign_profile_refused", code == 2)


MATRIX = {
    "C01": ("PARCIAL", "Literales de K 10.1 y la clase L (G0) alcanzan su regla con offsets en bytes sobre texto multibyte; "
                       "NO EVALUADO: parse-print-parse (la CLI no expone el AST ni el impresor)."),
    "C02": ("PASA", "utf8ref, T01-T14 y las 4 filas con use std.parse, familias V de String, O01-O04/O10/O11, "
                    "matriz de tamanos y escalares de K 12.2; ninguna normalizacion."),
    "C03": ("PASA", "X05-X10, X21, X27, N03/N04, P02/P05/P06, clase N de nominales, constructores y SCC."),
    "C04": ("PARCIAL", "X11, N05-N08, M04, O09, A05, A06 y las familias de propiedad; NO EVALUADO: conteo de "
                       "cierres por adquisicion comprometida en programas generados con recursos."),
    "C05": ("PARCIAL", "X23, N09 y familias no_alloc (literal, copia, Concat); NO EVALUADO: no_io de Print/ReadLine/"
                       "Println y destructores transitivos (K no da fila literal ni diagnostico fijo)."),
    "C06": ("PASA", "M01-M03, H12, H13, H12-edit, H13-move, P01, N01/N02, H11."),
    "C07": ("NO EVALUADO", "El verificador de IR y el acceso publico sin verificar no son observables por la CLI."),
    "C08": ("PARCIAL", "E01-E07, A08 y la clase E (35 familias de fallo del controlador, entornos validos) con "
                       "colas, eventos y bytes; NO EVALUADO: igualdad text/json (el oraculo compara solo JSON) y "
                       "agotamiento de cuota sin consumo de cola."),
    "C09": ("PARCIAL", "Recetas Q (L-1/L/L+1) en perfiles normales y los cuatro reducidos, X24-X27, O13, "
                       "fuzz sin panico; NO EVALUADO: perfiles reducidos de F5 y pilas de 256/512 KiB."),
    "C10": ("PASA", "A01-A08, aplicacion de K 11 con sus once variantes, el lote de cien lineas y entornos "
                    "aleatorios (clase A); sin deteccion por fuente."),
    "C11": ("NO EVALUADO", "Las regresiones de A2, F0-0003 y F0-0005 son de otras suites; el oraculo exige "
                           "perfil explicito y no toca la base."),
}


def matrix_report() -> dict:
    return {k: {"status": s, "note": n} for k, (s, n) in MATRIX.items()}


# ------------------------------------------------------------------ guards with labelled cases (mutation campaign)
# Each case states the rule it follows (K section) and expects what that text fixes, not what the model
# happens to return.

def _printed(program, file="g.n"):
    _, out, fault = m.run_program(program, file)
    return [utf8ref.from_bytes(b) for b in out.stdout], out, fault


def guards_text(rep: Report):
    probe = FN("P", [("x", "view", "I64")], [ES(C("Print", N("x"))), m.Ret(N("x"))], ret="I64")
    for op in ("<", "<=", ">", ">="):
        got, _, _ = _printed(prog(probe, main(LET("c", BIN(C("P", I(1)), op, C("P", I(2)))))))
        rep.check(f"K3.operands_evaluated_left_to_right.{op}", got == ["1", "2"], str(got))
    got, _, _ = _printed(prog(main(ES(C("Print", M(S(""), "IsEmpty"))), ES(C("Print", M(S("a"), "IsEmpty"))),
                                   ES(C("Print", M(S("é"), "IsEmpty"))))))
    rep.check("K3.IsEmpty_is_ByteLen_equal_zero", got == ["true", "false", "false"], str(got))
    got, _, _ = _printed(prog(main(ES(C("Print", M(S("ab"), "Concat", S("c")))),
                                   ES(C("Print", M(S("é"), "Concat", S("\U0001f642")))))))
    rep.check("K3.Concat_self_then_other", got == ["abc", "é\U0001f642"], str(got))
    fmt = prog(STRUCT("P", [("x", "I64"), ("y", "String")], derive=["Format"]), STRUCT("E0", [], derive=["Format"]),
               ENUM("E", [("A", []), ("B", ["I64", "String"])], derive=["Format"]),
               main(ES(C("Print", C("P", I(1), S("a")))), ES(C("Print", C("E0"))),
                    ES(C("Print", V("E", "A", null=True))), ES(C("Print", V("E", "B", I(1), S("x"))))))
    got, _, _ = _printed(fmt)
    rep.check("K5.2.derived_format_shapes", got == ["P(x: 1, y: a)", "E0()", "E.A", "E.B(1, x)"], str(got))
    only_format = prog(STRUCT("S", [], derive=["Format"]),
                       main(LET("a", C("S")), LET("b", C("S")), LET("c", BIN(N("a"), "==", N("b")))))
    d = m.check_program(only_format, "g.n")[1]
    rep.check("K5.2.Format_does_not_imply_PartialEq", len(d) == 1 and d[0]["code"] == "NX-E1403"
              and d[0]["subcause"] == "missing_partial_eq", str([(x["code"], x["subcause"]) for x in d]))
    only_eq = prog(STRUCT("S", [], derive=["PartialEq"]), main(LET("a", C("S")), ES(C("Print", N("a")))))
    d = m.check_program(only_eq, "g.n")[1]
    rep.check("K5.2.PartialEq_does_not_imply_Format", len(d) == 1 and d[0]["code"] == "NX-E1403"
              and d[0]["subcause"] == "missing_format", str([(x["code"], x["subcause"]) for x in d]))
    two_types = prog(STRUCT("S", []), STRUCT("T", []),
                     main(LET("a", C("S")), LET("b", C("T")), LET("c", BIN(N("a"), "==", N("b")))))
    d = m.check_program(two_types, "g.n")[1]
    rep.check("K5.2.different_types_fail_by_type_before_capability", len(d) == 1 and d[0]["code"] == "NX-E0102"
              and d[0]["subcause"] == "type_mismatch", str([(x["code"], x["subcause"]) for x in d]))


def guards_k3_k52_gaps(rep: Report):
    """Rules the model enforces but that no K row isolates (ADR 3, 5.2): each expects what K fixes."""
    sl = prog(main(LET("a", IDX(S("abc"), m.RNG(I(0), I(2))))))
    d = m.check_program(sl, "g.n")[1]
    rep.check("K3.string_range_selector_is_range_later_adr_alone",
              len(d) == 1 and d[0]["code"] == "NX-E0006" and d[0]["feature_id"] == "range"
              and d[0]["data"] == {"pending_scope": "later_adr"} and d[0]["stage"] == "types",
              str([(x["code"], x["feature_id"]) for x in d]))
    cp = prog(main(VAR("s", S("a")), ASG(N("s"), S("b"), "+=")))
    d = m.check_program(cp, "g.n")[1]
    rep.check("K3.compound_plus_on_String_is_type_mismatch_I64",
              len(d) == 1 and (d[0]["code"], d[0]["subcause"]) == ("NX-E0102", "type_mismatch")
              and d[0]["data"] == {"expected_type": "I64", "found_type": "String"}, str(d))
    ok = prog(main(VAR("n", I(1)), ASG(N("n"), I(2), "+=")))
    rep.check("K3.compound_plus_on_I64_is_accepted", m.check_program(ok, "g.n")[1] == [])
    pr = prog(main(ES(C("Print", C("MockOpen", I(1))))))
    d = m.check_program(pr, "g.n")[1]
    rep.check("K5.2.Print_of_a_resource_keeps_type_mismatch_not_E1403",
              len(d) == 1 and (d[0]["code"], d[0]["subcause"]) == ("NX-E0102", "type_mismatch"),
              str([(x["code"], x["subcause"]) for x in d]))
    st = STRUCT("S", [("x", "I64")])
    for label, body in (("nominal", [ES(C("AssertEq", N("a"), N("a")))]),
                        ("option_of_nominal", [LET("o", C("Some", N("a"))), ES(C("AssertEq", N("o"), N("o")))])):
        d = m.check_program(prog(st, main(LET("a", C("S", I(1))), *body)), "g.n")[1]
        rep.check(f"K5.2.AssertEq_requires_PartialEq.{label}",
                  len(d) == 1 and (d[0]["code"], d[0]["subcause"]) == ("NX-E1403", "missing_partial_eq")
                  and d[0]["data"] == {"required_capability": "PartialEq"}, str(d))
    st2 = STRUCT("S", [("x", "I64")], derive=["PartialEq"])
    d = m.check_program(prog(st2, main(LET("a", C("S", I(1))), ES(C("AssertEq", N("a"), N("a"))))), "g.n")[1]
    rep.check("K5.2.AssertEq_with_PartialEq_is_accepted", d == [])


def guards_nominal(rep: Report):
    guards_k3_k52_gaps(rep)
    two = prog(STRUCT("S", [("a", "I64"), ("b", "I64")]),
               main(LET("s", C("S", I(1), I(2))), ES(C("Print", FLD(N("s"), "a"))), ES(C("Print", FLD(N("s"), "b")))))
    got, _, _ = _printed(two)
    rep.check("K4.1.positional_arguments_follow_declaration_order", got == ["1", "2"], str(got))
    fail = FN("Fallar", [], [ES(C("Fail")), m.Ret(C("MockOpen", I(9)))], ret="MockResource")
    trio = prog(STRUCT("T", [("a", "MockResource"), ("b", "MockResource"), ("c", "MockResource")]), fail,
                main(LET("t", C("T", C("MockOpen", I(1)), C("MockOpen", I(2)), C("Fallar")))))
    _, out, _ = _printed(trio)
    rep.check("K4.4.incomplete_constructor_cleans_in_reverse_evaluation_order",
              out.events == ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
              and out.fault is not None and out.fault.code == "R0301", str(out.events))
    cyc = prog(STRUCT("A", [("b", "B"), ("c", "C")]), STRUCT("B", [("a", "A")]), STRUCT("C", [("d", "D")]),
               STRUCT("D", [("a", "A")]), main())
    src, d = m.check_program(cyc, "g.n")
    at = src.index("struct B{a:") + len("struct B{a:")
    rep.check("K4.2.shortest_cycle_primary_is_last_edge",
              len(d) == 1 and d[0]["code"] == "NX-E1323"
              and (d[0]["primary_span"]["start"], d[0]["primary_span"]["end"]) == (at, at + 1),
              str([(x["code"], x["primary_span"]) for x in d]))


def guards_literal(rep: Report):
    app_src = entry_map()["APP"]["source"]
    lits = literal.find_literals(app_src)
    rep.check("K11.application_literals_recognised_with_their_holes",
              all(r.other is None and r.diag is None for r in lits) and [r.holes for r in lits if r.holes] == [4, 3],
              str([(r.holes, r.other) for r in lits]))
    for esc, val in ((chr(34), chr(34)), (chr(92), chr(92)), ("n", chr(10)), ("r", chr(13)), ("t", chr(9))):
        r = literal.analyze('"x' + chr(92) + esc + 'y"', 0)
        text = "".join(p[1] for p in r.parts if p[0] == "text")
        rep.check(f"D15.2.4.escape_decodes.{ord(esc)}", r.other is None and text == "x" + val + "y", repr(text))
    two = barrier.literal_constructs((0, 5), 1)
    one = barrier.literal_constructs((0, 5), 0)
    rep.check("LF-02.class_B_needs_exactly_one_pending_K",
              barrier.class_b_eligible(two, ()) is False and barrier.class_b_eligible(one, ()) is True
              and barrier.class_b_eligible(two, ("K02",)) is True and barrier.class_b_eligible(one, ("K02",)) is False)
    expr = literal.analyze('"{a + b - c * d / e % f}"', 0)
    rep.check("K2.1.every_binary_operator_inside_a_hole", expr.other is None and expr.holes == 1, str(expr.other))
    for op in ("<", ">", "==", "!=", "<=", ">="):
        r = literal.analyze('"{a %s b}"' % op, 0)
        rep.check(f"K2.1.comparison_operator_inside_a_hole.{op}", r.other is None and r.holes == 1, str(r.other))
    res = literal.analyze('"a\rb"', 0)
    rep.check("literal.raw_cr_is_excluded_LF03", res.other == "raw_control_in_text", str(res.other))
    res = literal.analyze('"a\nb"', 0)
    rep.check("literal.raw_lf_is_excluded_LF03", res.other == "raw_control_in_text", str(res.other))
    src = entry_map()["X02"]["source"]
    found = literal.find_literals(src)
    rep.check("literal.recovery_yields_one_diagnostic_after_a_stray_brace",
              len([r for r in found if r.diag is not None]) == 1 and len(found) == 1,
              str([(r.start, r.end, r.diag) for r in found]))


def guards_model_limits(rep: Report):
    """K 8.2: the default quotas of the model evaluator at their boundary (limit passes, limit+1 faults)."""
    def lets(n, text=""):
        return prog(main(*[LET("s%d" % k, S(text)) for k in range(n)]))
    fault = lambda p: m.run_program(p, "g.n")[1].fault
    ok = fault(prog(main(LET("s", S("a" * 65536)))))
    bad = fault(prog(main(LET("s", S("a" * 65537)))))
    rep.check("K8.2.string_bytes_65536_passes_65537_faults",
              ok is None and bad is not None and bad.data == {"runtime_budget_kind": "string_bytes",
                                                              "limit": 65536, "observed": 65537}, str(bad and bad.data))
    ok, bad = fault(lets(4096)), fault(lets(4097))
    rep.check("K8.2.text_allocations_4096_passes_4097_faults",
              ok is None and bad is not None and bad.data["runtime_budget_kind"] == "text_allocations"
              and bad.data["observed"] == 4097 and bad.data["limit"] == 4096)
    chain = [LET("a", S("a" * 65536)), LET("b", N("a")), LET("c", N("b")), LET("d", N("c"))]
    ok = fault(prog(main(*chain)))
    bad = fault(prog(main(*chain, LET("e", S("x")))))
    rep.check("K8.2.live_text_bytes_262144_passes_262145_faults",
              ok is None and bad is not None and bad.data == {"runtime_budget_kind": "live_text_bytes",
                                                              "limit": 262144, "observed": 262145})


def guards_charges(rep: Report):
    guards_model_limits(rep)
    lim = dict(charges.NORMAL, live_text_bytes=10)
    in_loop = charges.account([("loop", 5, [("lit", 3, 0)])], lim)
    straight = charges.account([("lit", 3, 0), ("lit", 3, 1), ("lit", 3, 2), ("lit", 3, 3)], lim)
    rep.check("K8.2.strings_of_a_loop_body_die_at_the_end_of_each_iteration",
              in_loop.fault is None and in_loop.peak_live == 3 and in_loop.allocs == 5)
    rep.check("K8.2.strings_outside_a_loop_stay_alive",
              straight.fault is not None and straight.fault.quota == "live_text_bytes"
              and straight.fault.observed == 12 and straight.fault.span == 3)


def guards_harness(rep: Report):
    err_int = generic.Invocation(["x"], 1, {"outcome": {"kind": "err", "error": 70}, "stdout": []}, "", None, 0.0)
    want = {"exit": 1, "schema_version": 1, "outcome": {"kind": "err", "error": "70"}, "stdout": []}
    rep.check("harness.observed.err_as_integer_equals_decimal_string",
              runner.observed(err_int, [], want)["outcome"] == {"kind": "err", "error": "70"})
    ctl = generic.Invocation(["x"], 2, {"outcome": {"kind": "driver_error", "reason": "invalid_mock_json"},
                                        "stdout": ["x"], "events": []}, "", None, 0.0)
    want = {"exit": 2, "outcome": {"kind": "driver_error", "reason": "invalid_mock_json"}}
    got = runner.observed(ctl, [], want)
    rep.check("harness.observed.controller_document_is_compared_on_its_own_keys",
              got == want and generic._first_difference(want, got) is None, str(got))
    wrong = generic.Invocation(["x"], 2, {"outcome": {"kind": "driver_error", "reason": "invalid_mock_schema"}},
                               "", None, 0.0)
    rep.check("harness.observed.controller_reason_is_compared",
              generic._first_difference(want, runner.observed(wrong, [], want)) == "$.outcome.reason")


# ------------------------------------------------------------------ O6: the mutation harness itself

def check_mutate_units(rep: Report):
    """Copy, restoration by SHA-256, refusal of ambiguous patterns, verdicts and a two-worker campaign."""
    from . import mutate
    ST_TMP.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix=f"f4-oracle-st-mut-{os.getpid()}-", dir=str(ST_TMP)))
    try:
        rep.check("mutate.catalogue_valid", mutate.check_catalogue() == [] and len(mutate.MUTATIONS) >= 40,
                  str(mutate.check_catalogue()[:2]))
        ids = [m_.ident for m_ in mutate.MUTATIONS]
        rep.check("mutate.catalogue_ids_unique", len(ids) == len(set(ids)))
        rep.check("mutate.equivalents_carry_an_argument",
                  all(len(m_.equivalent) > 60 for m_ in mutate.MUTATIONS if m_.equivalent is not None))
        copy = mutate.Copy(work / "c").build()
        rel = "scripts/f4_oracle/barrier.py"
        original = generic.sha(Path(audit.ROOT) / rel)
        copy.apply(rel, "def compares_primary", "def compares_primary_x")
        changed = generic.sha(copy.root / rel)
        copy.restore(rel)
        rep.check("mutate.apply_changes_and_restore_returns_the_blob",
                  changed != original and generic.sha(copy.root / rel) == original and not copy.blobs)
        bad = []
        for pattern in ("zzz-not-in-the-file-zzz", "import"):
            try:
                copy.apply(rel, pattern, "x")
                bad.append(pattern)
                copy.restore(rel)
            except mutate.MutationError:
                pass
        rep.check("mutate.refuses_zero_or_several_occurrences", not bad, str(bad))
        copy.apply(rel, "def compares_primary", "def compares_primary_x")
        data, digest = copy.blobs[rel]
        copy.blobs[rel] = (data + b"#", digest)
        try:
            copy.restore(rel)
            refused = False
        except mutate.MutationError:
            refused = True
        copy.blobs.pop(rel, None)
        (copy.root / rel).write_bytes(data)
        rep.check("mutate.restoration_mismatch_aborts", refused)
        eq = mutate.Mutation("X", "f", rel, "a", "b", equivalent="argument " * 12)
        plain = mutate.Mutation("Y", "f", rel, "a", "b")
        rep.check("mutate.verdicts",
                  mutate.verdict(plain, {"status": "killed"}) == "killed"
                  and mutate.verdict(plain, {"status": "survived"}) == "SURVIVOR"
                  and mutate.verdict(eq, {"status": "survived"}) == "equivalent"
                  and mutate.verdict(eq, {"status": "killed"}) == "refuted")
        base = mutate.run_selftest(copy.root, "G0", 300)
        rep.check("mutate.unmutated_copy_passes", base["status"] == "survived", str(base))
        copy.close()
        picked = [m_ for m_ in mutate.MUTATIONS if m_.ident in ("B03", "L02")]
        code, results = mutate.run_campaign(picked, workers=2, timeout=300)
        rep.check("mutate.two_worker_campaign_kills_with_a_label",
                  code == 0 and all(r["verdict"] == "killed" and r["label"] for r in results)
                  and {r["id"] for r in results} == {"B03", "L02"}, str(results)[:200])
        left = [p for p in ST_TMP.glob(f"f4-oracle-mut-{os.getpid()}-*")]
        rep.check("mutate.campaign_removes_its_copies", not left, str(left))
    finally:
        shutil.rmtree(work, ignore_errors=True)


GROUPS = {"G0": run_g0, "G1": _rows_group("G1"), "G2": lambda rep: (run_model_basics(rep), _rows_group("G2")(rep), guards_text(rep)),
          "G3": lambda rep: (_rows_group("G3")(rep), run_scc_variants(rep), guards_nominal(rep)),
          "G4": _rows_group("G4"), "G5": _rows_group("G5"), "G6": _rows_group("G6"),
          "G7": _rows_group("G7"), "G8": lambda rep: (_rows_group("G8")(rep), run_semantics_extras(rep)),
          "G9": run_g9, "G10": run_g10, "GEN": run_gen}


def _sweep_own_dirs():
    """Remove the scratch directories of THIS process (``f4-oracle-st-<PID>-*``, ``f4-oracle-mut-<PID>-*``);
    a child that still holds a handle on Windows gets a few retries.  Other PIDs are never touched."""
    for pattern in (f"f4-oracle-st-{os.getpid()}-*", f"f4-oracle-mut-{os.getpid()}-*"):
        for path in Path("C:/nexo-work/tmp").glob(pattern):
            for _ in range(5):
                shutil.rmtree(path, ignore_errors=True)
                if not path.exists():
                    break
                time.sleep(0.5)


def _run_checks(rep: Report, fast: bool, groups):
    check_audit(rep)
    check_pins(rep)
    check_literal_rows(rep)
    check_diagnostic_contract(rep)
    check_utf8(rep, fast)
    if not fast:
        wanted = [g for g in (groups or list(GROUPS)) if g in GROUPS]
        for name in wanted:
            fn = GROUPS[name]
            _, violations = run_audited(lambda f=fn: f(rep))
            rep.check(f"audit.dynamic.{name}", not violations, "; ".join(violations[:4]))
        if groups is None or "GEN" in groups:
            check_gen_subprocess(rep)
        if groups is None or "HARNESS" in groups:
            run_harness(rep)
        if (groups is None or "MUTATE" in groups) and not os.environ.get("F4_ORACLE_MUTANT"):
            check_mutate_units(rep)  # a mutated copy does not run the campaign about itself
        probes = []
        _, v1 = run_audited(lambda: open(os.path.join(audit.ROOT, "Cargo.toml"), "rb").close())
        probes.append(bool(v1))
        _, v2 = run_audited(lambda: __import__("sched"))
        probes.append(bool(v2))
        rep.check("audit.dynamic.detects_violations", all(probes), str(probes))


def run(fast: bool = False, groups=None, fail_fast: bool = False) -> int:
    started = time.time()
    rep = Report(fail_fast)
    try:
        _run_checks(rep, fast, groups)
    except FailFast:
        pass
    finally:
        _sweep_own_dirs()
    for line in rep.lines:
        print(line)
    print(f"selftest: {rep.total} checks, {rep.failed} failed, {time.time() - started:.1f}s"
          + (" (fast)" if fast else "") + (" (fail-fast)" if fail_fast else ""))
    return 0 if rep.failed == 0 else 1
