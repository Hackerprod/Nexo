"""Self-test of the oracle (class ORQUESTA).

Order (plan 5): audit, pins and literal fragments, utf8ref against the host codec,
then the model groups G0..G8 under ``sys.addaudithook``.  Every expected value comes
from the pinned ADR (``fixtures_io``) or from the hand-written pairs; no binary is run.
"""

from __future__ import annotations

import json
import os
import random
import re
import sys
import time

from . import audit, barrier, diagnostics, fixtures_io as fio, literal, utf8ref

ARNES_MESSAGE = "selftest"


class Report:
    def __init__(self):
        self.lines: list[str] = []
        self.failed = 0
        self.total = 0

    def check(self, name: str, ok: bool, detail: str = "") -> bool:
        self.total += 1
        if ok:
            self.lines.append(f"PASS {name}")
        else:
            self.failed += 1
            self.lines.append(f"FAIL {name}: {detail}")
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


def check_diagnostic_contract(rep: Report):
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


GROUPS = {"G0": run_g0, "G1": _rows_group("G1"), "G2": lambda rep: (run_model_basics(rep), _rows_group("G2")(rep)),
          "G3": lambda rep: (_rows_group("G3")(rep), run_scc_variants(rep)),
          "G4": _rows_group("G4"), "G5": _rows_group("G5"), "G6": _rows_group("G6"),
          "G7": _rows_group("G7"), "G8": lambda rep: (_rows_group("G8")(rep), run_semantics_extras(rep)),
          "GEN": run_gen}


def run(fast: bool = False, groups=None) -> int:
    started = time.time()
    rep = Report()
    check_audit(rep)
    check_pins(rep)
    check_literal_rows(rep)
    check_diagnostic_contract(rep)
    check_utf8(rep, fast)
    if not fast:
        wanted = groups or list(GROUPS)
        for name in wanted:
            fn = GROUPS[name]
            _, violations = run_audited(lambda f=fn: f(rep))
            rep.check(f"audit.dynamic.{name}", not violations, "; ".join(violations[:4]))
        if groups is None or "GEN" in groups:
            check_gen_subprocess(rep)
        probes = []
        _, v1 = run_audited(lambda: open(os.path.join(audit.ROOT, "Cargo.toml"), "rb").close())
        probes.append(bool(v1))
        _, v2 = run_audited(lambda: __import__("sched"))
        probes.append(bool(v2))
        rep.check("audit.dynamic.detects_violations", all(probes), str(probes))
    for line in rep.lines:
        print(line)
    print(f"selftest: {rep.total} checks, {rep.failed} failed, {time.time() - started:.1f}s"
          + (" (fast)" if fast else ""))
    return 0 if rep.failed == 0 else 1
