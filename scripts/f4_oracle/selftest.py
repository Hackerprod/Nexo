"""Self-test of the oracle (class ORQUESTA).

Order (plan 5): audit, pins and literal fragments, utf8ref against the host codec,
then the model groups G0..G8 under ``sys.addaudithook``.  Every expected value comes
from the pinned ADR (``fixtures_io``) or from the hand-written pairs; no binary is run.
"""

from __future__ import annotations

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


def run_g1_g2_integrity(rep: Report):
    """O1: G1/G2 rows are present and structurally sound; the model pairs arrive with O2."""
    emap = entry_map()
    need = ["X04", "X23", "X24", "X25", "X26", "N09", "A01", "T01", "T10", "T14", "X22", "M05", "O01-<",
            "O02-!=", "O03->=", "O04", "O10", "O11", "O13", "B01-text", "A02"]
    missing = [n for n in need if n not in emap]
    rep.check("G1G2.rows_present", not missing, ",".join(missing))


GROUPS = {"G0": run_g0, "G1": run_g1_g2_integrity}


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
