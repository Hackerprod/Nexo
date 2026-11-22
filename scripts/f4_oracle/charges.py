"""Quota recipes of K 8.2 / 8.3 (class Q) and their logical accounting (class MODELO).

The model counts the *logical* allocations of K 8.2 (not a malloc counter): every string literal,
copy, ``Concat``, ... starts exactly one buffer (also the empty one); the quotas are checked in the
order of K 8.2 (``string_bytes``, ``live_text_bytes``, ``text_allocations``) when a buffer is
started; strings declared in a loop body die at the end of the iteration, the others at the end of
``Main``.  ``account`` predicts which quota is reached first, with which ``observed`` value and at
which source span; ``occupancy`` records every quota at that point (K 8.3: "demuestra que llega
primero a la objetivo").  ``nominal_members`` (K 8.1) counts one member per field.

Small cases are cross-checked in the self-test against the evaluator of ``model``; the closed form
here is what allows the 65,537-byte, 4,097-allocation and 16,385-member recipes.
"""

from __future__ import annotations

import random

from . import diagnostics, utf8ref
from .generate import PROFILE, _doc, checked_doc, rejected_doc, sha_hex

FILE = "case.n"
NORMAL = {"string_bytes": 65536, "live_text_bytes": 262144, "text_allocations": 4096, "nominal_members": 16384}
TEST_PREFIX = "f0-0004-b-test-"
REDUCED = {  # K 8.3: a single quota changes, the others keep the normal limit
    "string_bytes": (TEST_PREFIX + "string-8-v1", 8),
    "live_text_bytes": (TEST_PREFIX + "text-live-16-v1", 16),
    "text_allocations": (TEST_PREFIX + "text-alloc-2-v1", 2),
    "nominal_members": (TEST_PREFIX + "members-2-v1", 2),
}
ORDER = ("string_bytes", "live_text_bytes", "text_allocations")
ALL_FIELDS = list(diagnostics.FIELDS)
NO_PRIMARY = [f for f in diagnostics.FIELDS if f != "primary_span"]


def limits_of(profile: str) -> dict:
    lim = dict(NORMAL)
    for quota, (pid, value) in REDUCED.items():
        if pid == profile:
            lim[quota] = value
    return lim


# ------------------------------------------------------------------ source builder with byte spans

class Src:
    def __init__(self):
        self.parts: list = []
        self.size = 0

    def add(self, text: str):
        data = utf8ref.text(text)
        start = self.size
        self.parts.append(data)
        self.size += len(data)
        return (start, self.size)

    def text(self) -> str:
        return utf8ref.from_bytes(b"".join(self.parts))


def lit(text: str) -> str:
    return '"' + text + '"'


# ------------------------------------------------------------------ logical accounting

class Fault:
    def __init__(self, quota, limit, observed, span):
        self.quota, self.limit, self.observed, self.span = quota, limit, observed, span


class State:
    def __init__(self, limits):
        self.limits = limits
        self.live = 0
        self.allocs = 0
        self.max_string = 0
        self.peak_live = 0
        self.fault = None

    def start(self, nbytes, span):
        lim = self.limits
        if nbytes > lim["string_bytes"]:
            self.fault = Fault("string_bytes", lim["string_bytes"], nbytes, span)
        elif self.live + nbytes > lim["live_text_bytes"]:
            self.fault = Fault("live_text_bytes", lim["live_text_bytes"], self.live + nbytes, span)
        elif self.allocs + 1 > lim["text_allocations"]:
            self.fault = Fault("text_allocations", lim["text_allocations"], self.allocs + 1, span)
        if self.fault is not None:
            return False
        self.allocs += 1
        self.live += nbytes
        self.max_string = max(self.max_string, nbytes)
        self.peak_live = max(self.peak_live, self.live)
        return True


def account(ops, limits: dict) -> State:
    """``ops``: ("lit", nbytes, span), ("copy", index_of_source_op, span) and
    ("loop", count, [("lit", nbytes, span), ...]) (loop strings die at the end of each iteration)."""
    st = State(limits)
    sizes = []
    for op in ops:
        if op[0] == "lit":
            if not st.start(op[1], op[2]):
                return st
            sizes.append(op[1])
        elif op[0] == "copy":
            if not st.start(sizes[op[1]], op[2]):
                return st
            sizes.append(sizes[op[1]])
        else:
            for _ in range(op[1]):
                taken = 0
                for sub in op[2]:
                    if not st.start(sub[1], sub[2]):
                        return st
                    taken += sub[1]
                st.live -= taken
    return st


def occupancy(st: State, ops_count_members: int = 0) -> dict:
    return {"string_bytes": st.max_string, "live_text_bytes": st.peak_live, "text_allocations": st.allocs,
            "nominal_members": ops_count_members}


# ------------------------------------------------------------------ recipes

class Recipe:
    def __init__(self, name, source, profile, verbs, expected, target, occupancy_, long_):
        self.name, self.source, self.profile, self.verbs = name, source, profile, verbs
        self.expected, self.target, self.occupancy, self.long = expected, target, occupancy_, long_


def _r0002(fault: Fault, profile, with_primary=True):
    d = diagnostics.make("R0002", file=FILE, primary=fault.span, profile_id=profile,
                         data={"runtime_budget_kind": fault.quota, "limit": fault.limit,
                               "observed": fault.observed})
    fields = ALL_FIELDS if with_primary else NO_PRIMARY
    return {"fields": fields, "check": checked_doc(profile),
            "run": _doc(profile, 1, {"kind": "simulator_fault"}, [], [], [], [d], fields)}


def _unit(profile):
    return {"fields": [], "check": checked_doc(profile),
            "run": _doc(profile, 0, {"kind": "unit"}, [], [], [], [], [])}


def _text_recipe(name, build, profile, target, long_):
    """``build(src)`` appends the program and returns the accounting ops; the fault (if any) fixes
    the expected document; ``with_primary`` is dropped for copy faults (LAGUNAS LF-24)."""
    src = Src()
    ops, copy_primary = build(src)
    st = account(ops, limits_of(profile))
    if st.fault is None:
        expected = _unit(profile)
    else:
        expected = _r0002(st.fault, profile, with_primary=not copy_primary(st.fault))
    return Recipe(name, src.text(), profile, ["check", "run"], expected, st.fault.quota if st.fault else None,
                  occupancy(st), long_)


def filler(rng, nbytes: int) -> str:
    """Text of exactly ``nbytes`` UTF-8 bytes (ASCII with some two-byte letters)."""
    out, left = [], nbytes
    while left > 0:
        if left >= 2 and rng.random() < 0.1:
            out.append("é")
            left -= 2
        else:
            out.append(rng.choice("abcdefghijklmnopqrstuvwxyz0123456789 "))
            left -= 1
    return "".join(out)


def r_string_literal(rng, nbytes, profile=PROFILE, long_=False, name="string_literal"):
    text = filler(rng, nbytes)
    spans = {}

    def build(src):
        src.add("fn Main(){let s=")
        span = src.add(lit(text))
        src.add(";}")
        return [("lit", nbytes, span)], lambda f: False
    return _text_recipe(name, build, profile, "string_bytes", long_)


def r_live_chain(rng, size, copies, extra=0, profile=PROFILE, long_=False):
    """A literal of ``size`` bytes, ``copies`` successive copies (``let b=a;`` ...) and optionally a
    final small literal of ``extra`` bytes (the exact boundary of live_text_bytes)."""
    names = ["a", "b", "c", "d", "e", "f", "g", "h"]
    text = filler(rng, size)

    def build(src):
        ops = []
        src.add("fn Main(){let a=")
        ops.append(("lit", size, src.add(lit(text))))
        src.add(";")
        for k in range(copies):
            src.add("let %s=" % names[k + 1])
            ops.append(("copy", k, src.add(names[k])))
            src.add(";")
        if extra:
            src.add("let z=")
            ops.append(("lit", extra, src.add(lit(filler(rng, extra)))))
            src.add(";")
        src.add("}")
        return ops, lambda f: any(o[0] == "copy" and o[2] == f.span for o in ops)
    return _text_recipe("live_chain", build, profile, "live_text_bytes", long_)


def r_alloc_loop(rng, count, per_iteration, profile=PROFILE, long_=False):
    """``for i in 1..=count`` creating ``per_iteration`` small strings per iteration, ``n`` an I64 counter."""
    body_texts = [rng.choice(["", "", "a", "é", "xy"]) for _ in range(per_iteration)]
    tail = rng.choice(["n+=1;", "n=n+1;"])

    def build(src):
        src.add("fn Main(){var n=0;for i in 1..=%d{" % count)
        body = []
        for k, text in enumerate(body_texts):
            src.add("let s%d=" % k)
            body.append(("lit", len(utf8ref.text(text)), src.add(lit(text))))
            src.add(";")
        src.add(tail + "}}")
        return [("loop", count, body)], lambda f: False
    return _text_recipe("alloc_loop", build, profile, "text_allocations", long_)


def r_members(rng, count, profile=PROFILE, long_=False):
    """A struct of ``count`` I64 fields, never constructed (K 8.3)."""
    limit = limits_of(profile)["nominal_members"]
    prefix = rng.choice(["a", "f", "x", "m", "v"])
    src = Src()
    src.add("struct S{")
    spans = []
    for i in range(count):
        span = src.add("%s%d" % (prefix, i))
        spans.append(span)
        src.add(":I64;")
    src.add("}")
    if count > limit:
        d = diagnostics.make("NX-E0003", file=FILE, primary=spans[limit], profile_id=profile,
                             stage="types", operation="analysis.limit",
                             data={"budget_kind": "nominal_members", "budget_scope": "compilation",
                                   "incomplete_analysis": "types", "limit": limit, "observed": limit + 1})
        expected = {"fields": ALL_FIELDS, "check": rejected_doc(profile, [d], ALL_FIELDS)}
        target = "nominal_members"
    else:
        expected = {"fields": [], "check": checked_doc(profile)}
        target = None
    occ = {"string_bytes": 0, "live_text_bytes": 0, "text_allocations": 0, "nominal_members": min(count, limit + 1)}
    return Recipe("members", src.text(), profile, ["check"], expected, target, occ, long_)


def recipes(seed: int, final: bool) -> list:
    """The recipes of one seed.  Without ``final`` the failing normal-profile recipes of K 8.3 (65,537
    bytes, 4,097 allocations, 16,385 members, five strings of 60,000 bytes) are not run; the passing
    boundaries and every reduced-profile recipe are."""
    rng = random.Random("f4o-q/%d" % seed)
    out = []
    # string_bytes: 65,535 / 65,536 pass, 65,537.. fail
    for n in (65535, 65536) + ((65537, 65537 + seed) if final else ()):
        out.append(r_string_literal(rng, n, long_=n > NORMAL["string_bytes"]))
    # live_text_bytes: four strings of 60,000 pass, the fifth proposes 300,000
    out.append(r_live_chain(rng, 60000, 3))
    out.append(r_live_chain(rng, 65536, 3))                        # exactly 262,144
    if seed:
        out.append(r_live_chain(rng, 65536 - seed, 3, extra=4 * seed))  # 262,144 reached exactly
    if final:
        out.append(r_live_chain(rng, 60000, 4, long_=True))
        out.append(r_live_chain(rng, 65536, 3, extra=1, long_=True))
        out.append(r_live_chain(rng, 65536 - seed, 3, extra=4 * seed + 1, long_=True))
    # text_allocations: 4,095 / 4,096 pass, 4,097 fails (also with two strings per iteration)
    for count, per in ((4095, 1), (4096, 1), (2048, 2), (1365, 3)):
        out.append(r_alloc_loop(rng, count, per))
    if final:
        for count, per in ((4097, 1), (2049, 2), (1366, 3)):
            out.append(r_alloc_loop(rng, count, per, long_=True))
    # nominal_members: 16,383 / 16,384 pass, 16,385 fails
    out.append(r_members(rng, 16383 - seed))
    out.append(r_members(rng, 16384))
    if final:
        out.append(r_members(rng, 16385, long_=True))
    # reduced profiles: one quota each (K 8.3), around the limit
    pid, lim = REDUCED["string_bytes"]
    for n in (0, lim - 1, lim, lim + 1, lim + 1 + seed):
        out.append(r_string_literal(rng, n, pid, name="string_8"))
    pid, lim = REDUCED["live_text_bytes"]
    for sizes in ((8, 8, 0), (8, 8, 1), (16,), (17,), (5, 5, 5, 2), (4, 4, 4, 4, 1)):
        out.append(_live_literals(rng, sizes, pid))
    pid, lim = REDUCED["text_allocations"]
    for n in (0, 1, 2, 3, 4):
        out.append(_alloc_literals(rng, n, pid))
    pid, lim = REDUCED["nominal_members"]
    for n in (0, 1, 2, 3, 5):
        out.append(r_members(rng, n, pid))
    return out


def _live_literals(rng, sizes, profile):
    def build(src):
        src.add("fn Main(){")
        ops = []
        for k, n in enumerate(sizes):
            src.add("let v%d=" % k)
            ops.append(("lit", n, src.add(lit(filler(rng, n)))))
            src.add(";")
        src.add("}")
        return ops, lambda f: False
    return _text_recipe("live_16", build, profile, "live_text_bytes", False)


def _alloc_literals(rng, n, profile):
    def build(src):
        src.add("fn Main(){")
        ops = []
        for k in range(n):
            src.add("let v%d=" % k)
            ops.append(("lit", 0, src.add(lit(""))))
            src.add(";")
        src.add("}")
        return ops, lambda f: False
    return _text_recipe("alloc_2", build, profile, "text_allocations", False)


def q_records(seed: int, final: bool, first_index: int = 0) -> list:
    out, seen = [], set()
    for r in recipes(seed, final):
        key = sha_hex(r.source + "\0" + r.profile)
        if key in seen:
            continue
        seen.add(key)
        family = "q_" + r.name + ("_" + r.profile[len(TEST_PREFIX):] if r.profile != PROFILE else "")
        out.append({"seed": seed, "index": first_index + len(out), "role": "main", "cls": "Q", "family": family,
                    "fclass": "Q", "phase": "G9", "file": FILE, "profile_id": r.profile,
                    "source_hex": r.source.encode("utf-8").hex(), "verbs": r.verbs, "expected": r.expected,
                    "expected_sha256": _sha(r.expected), "repr": None, "distinct": key,
                    "structural": sha_hex("Q|" + family + "|" + str(r.target)), "target": r.target,
                    "occupancy": r.occupancy, "long": r.long})
    return out


def _sha(expected):
    from . import fixtures_io as fio
    return fio.sha256_text(fio.dumps(expected))
