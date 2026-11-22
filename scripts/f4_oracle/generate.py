"""Generators of the F4 oracle (class MODELO): classes V, N, L and B.

Every case is described by a ``rep`` (the oracle's own typed representation, plain
lists/dicts) from which the source and the expected result are derived by the model
(``expect``).  V = valid programs, N = one isolated mutation of a valid program (with its
repaired positive), L = grammar of the literal Hole (one subcause per source), B = one
construction of a capability that is still pending (K 9.1).  Expected values come only from
K through ``model``, ``literal``, ``barrier`` and ``diagnostics``; nothing is read from a
binary, from the compiler sources or from a prototype.

``reductions(rep)`` feeds the minimizer; ``structural`` is the structural form of a case
(literals normalised, identifiers renamed by position).
"""

from __future__ import annotations

import hashlib
import random
from dataclasses import dataclass, is_dataclass

from . import barrier, diagnostics, fixtures_io as fio, literal, model, utf8ref
from .model import (ARM, ARR, ASG, B, BIN, C, ES, EXT, FN, FOR, F, I, IDX, IFE, LET, M, MATCH, METH,
                    N, S, STRUCT, ENUM, V, VAR, Program, Ret, UseExt, Use, RawDecl, PVar, PBind,
                    PWild, Mv, TRY, edit, lab, move)

PROFILE = diagnostics.PROFILE
FILE = "case.n"
SEED_LABEL = "f4o"

# ------------------------------------------------------------------ field groups (plan 3)

GROUPS = {
    "C": ("schema_version", "diagnostic_namespace", "profile_id", "code", "subcause", "rule",
          "explanation_id", "severity", "stage", "operation", "feature_id", "repair"),
    "D": ("data",),
    "P": ("primary_span",),
    "Rel": ("related_spans",),
    "T": ("unexpected_token", "expected_tokens", "expected_tokens_kind", "parser_context",
          "recognized_prefix"),
    "X": ("cause_chain",),
}


def fields_of(groups: str) -> list:
    out = []
    for g in groups.split():
        out += list(GROUPS[g])
    return out


def project(diag: dict, names) -> dict:
    return {k: diag[k] for k in names}


# ------------------------------------------------------------------ pools

VARS = ["a", "b", "c", "d", "e", "f", "g", "h", "k", "m", "n", "p", "q", "r", "s", "t", "u", "v",
        "w", "x", "y", "z", "val", "tmp", "acc", "cnt", "idx", "res", "out", "buf", "line", "word",
        "item", "total", "left", "right", "first", "last", "size", "mark"]
TYS = ["S", "T", "P", "Pt", "Box", "Caja", "Dato", "Nodo", "Item", "Rec", "Cfg", "Par", "Reg",
       "Obj", "Punto", "Lista", "Marca", "Celda"]
FLDS = ["x", "y", "z", "w", "id", "num", "tag", "kind", "a", "b", "c", "val", "name", "count"]
FNS = ["F", "G", "H", "Run", "Calc", "Foo", "Util", "Work", "Paso", "Tarea", "Medir"]
VNTS = ["A", "B", "C", "D", "V", "W", "Una", "Dos", "Tres", "Uno", "Nada", "Dato", "Fin", "Inicio"]
MTHS = ["F", "G", "Get", "Set", "Add", "Inc", "Mide", "Paso"]
UNKNOWN_METHODS = ["Nope", "Foo", "Upper", "Trim", "Split", "Reverse", "Lower", "Count"]

# Texts without raw control characters (LF-03), braces, quotes or backslashes.
TEXTS = ["", "a", "b", "abc", "hello", "é", "ñ", "ü", "日本", "🙂", "é", "É", "Ω", "z", "zeta",
         "über", "naïve", "x y", "a-b", "10", "-5", "+1", "0007", "١", "-", "1x", "߿",
         "ࠀ", "퟿", "", "￿", "\U00010000", "\U0010ffff", "é🙂", "ab ", " ab",
         "Nexo", "línea", "tab", "A1", "999", "-0"]
ESC_TEXTS = ["a\nb", "x\ty", "q\"uote", "back\\slash", "{", "}", "{{", "}}", "a{b", "c}d", "\r", "\n"]
NUMS = ["0", "1", "-1", "7", "42", "-42", "0007", "+1", " 1", "1 ", "-0", "-", "", "1x", "٣",
        "9223372036854775807", "9223372036854775808", "-9223372036854775808",
        "-9223372036854775809", "999999999999999999999x"]
OPS6 = ["<", "<=", ">", ">=", "==", "!="]
PHASES = tuple("G%d" % i for i in range(11))


class Gen:
    """Per-case random source: names, values and padding (``random.Random`` only picks)."""

    def __init__(self, rng):
        self.r = rng
        self.taken = set()

    def _name(self, pool):
        n = self.r.choice(pool)
        while n in self.taken:
            n = n + str(self.r.randint(1, 99))
        self.taken.add(n)
        return n

    def var(self): return self._name(VARS)
    def ty(self): return self._name(TYS)
    def fld(self): return self._name(FLDS)
    def fn(self): return self._name(FNS)
    def vnt(self): return self._name(VNTS)
    def mth(self): return self._name(MTHS)

    def i(self, lo=0, hi=20):
        return I(self.r.randint(lo, hi))

    def text(self, escapes=False):
        pool = TEXTS + ESC_TEXTS if escapes and self.r.random() < 0.35 else TEXTS
        return self.r.choice(pool)

    def sep(self):
        return self.r.choice(["", "", "\n", " ", "\r\n"])

    def iexpr(self, names, depth=2):
        r = self.r
        t = r.random()
        if depth <= 0 or t < 0.3:
            return I(r.randint(0, 20)) if (not names or r.random() < 0.5) else N(r.choice(names))
        if t < 0.85:
            return BIN(self.iexpr(names, depth - 1), r.choice(["+", "-", "*"]),
                       self.iexpr(names, depth - 1), sp=r.random() < 0.5)
        cond = BIN(self.iexpr(names, 0), r.choice(OPS6), self.iexpr(names, 0), sp=True)
        return IFE(cond, self.iexpr(names, depth - 1), self.iexpr(names, depth - 1))

    def pad(self):
        out = []
        for _ in range(self.r.choice([0, 1, 2, 3])):
            k = self.r.randrange(4)
            v = self.var()
            if k == 0:
                out.append(LET(v, self.i()))
            elif k == 1:
                out.append(LET(v, B(self.r.random() < 0.5)))
            elif k == 2:
                out.append(LET(v, BIN(self.i(), self.r.choice("+-*"), self.i(), sp=self.r.random() < 0.5)))
            else:
                out.append(VAR(v, self.i()))
        return out


PAD_FORMS = 85          # 1 + 4 + 16 + 64 structural paddings (0..3 statements of 4 kinds)


def _main(*body, ret=None):
    return FN("Main", [], list(body), ret=ret)


def _prog(decls, g):
    return Program(list(decls), sep=g.sep())


def _pr(*args):
    return ES(C("Print", *args))


# ------------------------------------------------------------------ construction table (barrier)

STRING_METHODS = {"ByteLen": "string_bytelen", "IsEmpty": "string_is_empty",
                  "ScalarLen": "string_scalar_len", "Concat": "string_concat",
                  "SliceBytes": "string_slice_bytes", "Scalars": "string_scalars"}
QUALIFIED = {"std.text.FromI64": "std_text_from_i64", "text.FromI64": "std_text_from_i64",
             "std.parse.I64": "std_parse_i64", "parse.I64": "std_parse_i64"}


def _span_of(rendered, node):
    for slot in ("all", "name", "callee", "path", "kw", "attr", "op"):
        key = (id(node), slot)
        if key in rendered.spans:
            return rendered.spans[key]
    return (0, 0)


def constructs_of(prog, rendered):
    """Syntactic constructions of a program, as ``barrier.Construct`` in source order.

    Returns (constructs, ambiguous) where ``ambiguous`` is true when the program holds a
    construction whose K is ambiguous in K (LF-02), so it is excluded from class B.
    """
    out = []
    amb = []
    structs = {d.name for d in prog.decls if isinstance(d, (model.Struct, model.Enum))}
    strnames = set()
    for node in _walk(prog):
        if isinstance(node, model.Let):
            if isinstance(node.init, model.Str) or (node.ty is not None and node.ty.name == "String"):
                strnames.add(node.name)
        if isinstance(node, model.Param) and node.t is not None and node.t.name == "String":
            strnames.add(node.name)

    def add(kind, node):
        out.append(barrier.Construct(kind, tuple(_span_of(rendered, node))))

    def is_str(e):
        return (isinstance(e, model.Str) or (isinstance(e, model.Name) and e.name in strnames)
                or (isinstance(e, model.MCall) and e.name == "Concat")
                or (isinstance(e, model.Call) and e.callee in ("std.text.FromI64", "text.FromI64")))

    def tref(t):
        if t is None:
            return
        if t.name == "String":
            add("string_type", t)
        if t.name == "Array" and any(a.name == "String" for a in t.args):
            add("array_string", t)
        for a in t.args:
            tref(a)

    for node in _walk(prog):
        if isinstance(node, model.Str):
            add("string_literal", node)
            if any(not isinstance(p, str) for p in node.parts):
                add("string_hole", node)
        elif isinstance(node, model.TRef):
            tref(node)
        elif isinstance(node, model.Struct):
            add("struct_decl", node)
            for f in node.fields:
                if f.t.name == "MockResource":
                    add("resource_in_nominal", f.t)
            if node.derives:
                add("derive_attr", node)
            if node.pub:
                add("pub_decl", node)
        elif isinstance(node, model.Enum):
            add("enum_decl", node)
            for v in node.variants:
                for t in v.payload:
                    if t.name == "MockResource":
                        add("resource_in_nominal", t)
            if node.derives:
                add("derive_attr", node)
            if node.pub:
                add("pub_decl", node)
        elif isinstance(node, model.Fn):
            if node.pub:
                add("pub_decl", node)
            if node.derives:
                add("derive_attr", node)
        elif isinstance(node, model.Extend):
            add("extend_decl", node)
        elif isinstance(node, model.UseExt):
            add("use_extension", node)
        elif isinstance(node, model.Use):
            if node.pub:
                add("pub_use", node)
            elif node.alias:
                add("use_alias", node)
            else:
                add("use_import", node)
        elif isinstance(node, model.Field_):
            add("field_access", node)
        elif isinstance(node, model.Var):
            add("nominal_constructor", node)
        elif isinstance(node, model.Call):
            if node.callee in structs:
                add("nominal_constructor", node)
            elif node.callee in QUALIFIED:
                add(QUALIFIED[node.callee], node)
            elif node.callee.startswith("std.console."):
                add("console_call", node)
            elif node.callee == "Print" and node.args and is_str(node.args[0].expr):
                add("string_print", node)
            elif node.callee == "Print" and node.args and isinstance(node.args[0].expr, model.Name) \
                    and node.args[0].expr.name not in strnames:
                pass
        elif isinstance(node, model.MCall):
            if node.name in STRING_METHODS:
                add(STRING_METHODS[node.name], node)
            elif is_str(node.recv):
                # an unknown member of String is a text operation (K03, NX-E1402), not a user method
                add("string_scalar_len", node)
            else:
                add("method_call", node)
        elif isinstance(node, model.Bin):
            if is_str(node.left) or is_str(node.right):
                add("string_eq" if node.op == "==" else "string_ne" if node.op == "!="
                    else "string_order", node)
        elif isinstance(node, model.Arr):
            if any(isinstance(x, model.Str) for x in node.items):
                add("array_string", node)
            if any(isinstance(x, model.Call) and x.callee in structs for x in node.items):
                add("array_nominal", node)
        elif isinstance(node, model.For):
            if isinstance(node.iter, model.Arr) and any(isinstance(x, model.Str) for x in node.iter.items):
                add("for_string", node)
            elif isinstance(node.iter, model.Str):
                add("for_string", node)
        elif isinstance(node, model.MatchS):
            for arm in node.arms:
                if isinstance(arm.pattern, model.PVar):
                    add("nominal_pattern", arm.pattern)
    out.sort(key=lambda c: c.span)
    return out, bool(amb)


def _walk(node):
    """Pre-order walk over the dataclass graph of a program."""
    stack = [node]
    while stack:
        x = stack.pop()
        if is_dataclass(x) and not isinstance(x, type):
            yield x
            kids = []
            for v in vars(x).values():
                kids.append(v)
            stack.extend(reversed(kids))
        elif isinstance(x, (list, tuple)):
            stack.extend(reversed(list(x)))


# ------------------------------------------------------------------ rep <-> Program

_CLASSES = {name: cls for name, cls in vars(model).items()
            if isinstance(cls, type) and is_dataclass(cls)}


def to_repr(x):
    if is_dataclass(x) and not isinstance(x, type):
        d = {"_": type(x).__name__}
        for k, v in vars(x).items():
            d[k] = to_repr(v)
        return d
    if isinstance(x, tuple):
        return {"_t": [to_repr(i) for i in x]}
    if isinstance(x, list):
        return [to_repr(i) for i in x]
    return x


def from_repr(x):
    if isinstance(x, dict):
        if "_t" in x:
            return tuple(from_repr(i) for i in x["_t"])
        kw = {k: from_repr(v) for k, v in x.items() if k != "_"}
        if x["_"] == "Program" and "breaks" in kw:
            kw["breaks"] = tuple(kw["breaks"])
        return _CLASSES[x["_"]](**kw)
    if isinstance(x, list):
        return [from_repr(i) for i in x]
    return x


def clone_prog(prog):
    return from_repr(to_repr(prog))


# ------------------------------------------------------------------ structural form

def _user_names(prog):
    users = set()
    for n in _walk(prog):
        if isinstance(n, (model.Struct, model.Enum, model.Fn)):
            users.add(n.name)
        if isinstance(n, model.Fld) or isinstance(n, model.Vnt) or isinstance(n, model.Param):
            users.add(n.name)
        if isinstance(n, model.Let):
            users.add(n.name)
        if isinstance(n, model.For):
            users.add(n.var)
        if isinstance(n, model.PBind):
            users.add(n.name)
        if isinstance(n, model.Use) and n.alias:
            users.add(n.alias)
    users.discard("Main")
    users.discard("self")
    return users


def structural_program(prog) -> str:
    users = _user_names(prog)
    names = {}
    out = []

    def ren(s):
        if s not in names:
            names[s] = "n%d" % len(names)
        return names[s]

    def go(x):
        if is_dataclass(x) and not isinstance(x, type):
            tag = type(x).__name__
            out.append(tag + "(")
            if isinstance(x, model.Lit):
                v = x.value
                out.append("b" if isinstance(v, bool) else "N" if isinstance(v, int) else "u")
            elif isinstance(x, model.Str):
                for p in x.parts:
                    if isinstance(p, str):
                        out.append("S")
                    else:
                        go(p)
            else:
                for k, v in vars(x).items():
                    if tag == "Program" and k in ("sep", "final_lf", "breaks", "domain"):
                        continue
                    out.append(k + "=")
                    go(v)
            out.append(")")
        elif isinstance(x, (list, tuple)):
            out.append("[")
            for i in x:
                go(i)
                out.append(",")
            out.append("]")
        elif isinstance(x, str):
            out.append(ren(x) if x in users else x)
        else:
            out.append(repr(x))
        out.append(";")

    go(prog)
    return "".join(out)


def sha_hex(text) -> str:
    if isinstance(text, str):
        text = text.encode("utf-8")
    return hashlib.sha256(text).hexdigest()


# ------------------------------------------------------------------ result documents

def _doc(profile, exit_code, outcome, stdout, events, order, diags, fields):
    obs = [{"channel": ch, "text": tx} for ch, tx in order]
    nbytes = 0
    for ch, tx in order:
        n = len(utf8ref.text(tx))
        nbytes += (8 + n + 1) if ch == "event" else (n + 1)
    return {"exit": exit_code, "schema_version": 1, "profile_id": profile, "outcome": outcome,
            "stdout": stdout, "events": events, "observations": obs, "observation_bytes": nbytes,
            "reserved_observation_bytes": 0,
            "diagnostics": [project(d, fields) for d in diags]}


def rejected_doc(profile, diags, fields):
    return _doc(profile, 1, {"kind": "rejected"}, [], [], [], diags, fields)


def checked_doc(profile):
    return _doc(profile, 0, {"kind": "checked"}, [], [], [], [], [])


# ------------------------------------------------------------------ expectation from a rep

def lit_source(lit: dict) -> str:
    pads = ""
    if lit["pads"]:
        pads = "".join(_stmt_text(from_repr(p)) for p in lit["pads"])
    return "fn Main(){" + pads + lit["head"] + lit["body"] + lit["tail"] + "}"


def _stmt_text(stmt) -> str:
    src = model.render(Program([FN("Main", [], [stmt])])).source
    return src[len("fn Main(){"):-1]


def lit_positive_program(lit: dict):
    pads = [from_repr(p) for p in lit["pads"]]
    text = lit["positive_text"]
    stmt = LET(lit["var"], S(text)) if lit["ctx"] == "let" else _pr(S(text))
    return Program([_main(*pads, stmt)])


def expect(rep: dict, profile: str = PROFILE) -> dict:
    """Derive source, status and expected documents of a rep (status ``model_error`` when the
    model cannot evaluate it, e.g. a reduced program that lost a declaration)."""
    try:
        return _expect(rep, profile)
    except (KeyError, IndexError, TypeError, ValueError, AttributeError, ZeroDivisionError):
        return {"source": "", "status": "model_error", "expected": None, "diags": [], "scope": None}


def _expect(rep: dict, profile: str = PROFILE) -> dict:
    """Derive source, status and expected documents of a rep.

    Returns ``{"source", "status", "expected", "diags", "scope"}``; ``expected`` is the
    record that is compared (fields list, one document per verb).
    """
    cls = rep["cls"]
    closed = tuple(rep["closed"])
    verbs = tuple(rep["verbs"])
    if cls == "L":
        src = lit_source(rep["lit"])
        found = literal.first_diagnostic(src)
        if found is None or "other" in found:
            return {"source": src, "status": "open", "expected": None, "diags": [], "scope": None}
        d = diagnostics.make("NX-E1401", found["subcause"], file=FILE, primary=found["primary"],
                             related=[("delimiter", found["delimiter"])],
                             unexpected_token=(found["token"][0], found["token"][1], found["token"][2]),
                             profile_id=profile)
        fl = fields_of(rep["fields"])
        docs = {v: rejected_doc(profile, [d], fl) for v in verbs}
        return {"source": src, "status": "syntax", "expected": {"fields": fl, **docs}, "diags": [d],
                "scope": found["subcause"]}
    prog = from_repr(rep["program"])
    rendered = model.render(prog)
    src = rendered.source
    constructs, amb = constructs_of(prog, rendered)
    pend = barrier.pending(constructs, closed)
    if pend:
        if cls == "B" and not barrier.class_b_eligible(constructs, closed, ("x",) if amb else ()):
            return {"source": src, "status": "ineligible", "expected": None, "diags": [], "scope": None}
        chosen = pend[0][3]
        d = barrier.barrier_diagnostic(constructs, closed, FILE)
        d.pop("message", None)
        d["profile_id"] = profile
        gr = "C D P" if barrier.compares_primary(chosen.kind) else "C D"
        fl = fields_of(gr)
        docs = {v: rejected_doc(profile, [d], fl) for v in verbs}
        return {"source": src, "status": "barrier", "expected": {"fields": fl, **docs}, "diags": [d],
                "scope": pend[0][2], "kind": chosen.kind}
    if cls == "B":
        return {"source": src, "status": "no_barrier", "expected": None, "diags": [], "scope": None}
    _, diags = model.check_program(prog, FILE)
    fl = fields_of(rep["fields"])
    if diags:
        if any(d["code"].startswith("R") for d in diags):
            return {"source": src, "status": "fault", "expected": None, "diags": diags, "scope": None}
        for d in diags:
            d["profile_id"] = profile
        docs = {v: rejected_doc(profile, diags, fl) for v in verbs}
        return {"source": src, "status": "diag", "expected": {"fields": fl, **docs}, "diags": diags,
                "scope": diags[0]["code"] + "/" + str(diags[0]["subcause"])}
    docs = {}
    if "check" in verbs:
        docs["check"] = checked_doc(profile)
    status = "ok"
    if "run" in verbs:
        _, out, fault = model.run_program(prog, FILE)
        if fault is not None or out.fault is not None:
            return {"source": src, "status": "fault", "expected": None, "diags": [], "scope": None}
        if out.main_kind not in ("unit", "ok"):
            return {"source": src, "status": "main_err", "expected": None, "diags": [], "scope": None}
        if out.stdout and out.events:
            return {"source": src, "status": "unordered", "expected": None, "diags": [], "scope": None}
        stdout = [utf8ref.from_bytes(b) for b in out.stdout]
        order = [("stdout", t) for t in stdout] + [("event", e) for e in out.events]
        docs["run"] = _doc(profile, 0, {"kind": out.main_kind}, stdout, list(out.events), order, [], [])
    return {"source": src, "status": status, "expected": {"fields": [], **docs}, "diags": [],
            "scope": None}


# ------------------------------------------------------------------ families

@dataclass(frozen=True)
class Fam:
    name: str
    cls: str                      # V, N, L, B
    needs: tuple                  # capabilities that must be closed
    build: object
    fields: str = "C"
    code: str = None
    sub: str = None
    max_forms: int = None         # None: unbounded
    verbs: tuple = ("check", "run")
    uses: tuple = ()              # B: capabilities used besides the pending one
    target: str = None            # B: the pending capability


V_FAMS: list = []
N_FAMS: list = []
L_FAMS: list = []
B_FAMS: list = []


def _reg(lst, **kw):
    def deco(fn):
        lst.append(Fam(name=fn.__name__, build=fn, **kw))
        return fn
    return deco


# ---- V: valid programs

@_reg(V_FAMS, cls="V", needs=("K01",))
def v_int(g):
    names, muts, body = [], [], []
    printed = False
    for _ in range(g.r.randint(1, 6)):
        t = g.r.random()
        if t < 0.25 or not names:
            v = g.var()
            body.append(LET(v, g.iexpr(names)))
            names.append(v)
        elif t < 0.4:
            v = g.var()
            body.append(VAR(v, g.i()))
            names.append(v)
            muts.append(v)
        elif t < 0.55 and muts:
            body.append(ASG(N(g.r.choice(muts)), g.iexpr(names, 1), g.r.choice(["=", "+=", "-="])))
        elif t < 0.85:
            body.append(_pr(g.iexpr(names)))
            printed = True
        else:
            n_ = g.r.choice(names)
            body.append(ES(C("AssertEq", N(n_), N(n_))))
    if not printed:
        body.append(_pr(g.iexpr(names)))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K01",))
def v_array_int(g):
    a = g.var()
    items = [g.i() for _ in range(g.r.randint(0, 4))]
    body = []
    if items and g.r.random() < 0.5:
        body.append(LET(a, ARR(*items)))
    else:
        body.append(VAR(a, ARR(*items), t="Array<I64>" if not items else None))
        for _ in range(g.r.randint(0, 2)):
            body.append(ES(C("Push", edit(N(a)), g.i())))
    x = g.var()
    kinds = g.r.randint(0, 3)
    if kinds == 0:
        body.append(FOR(x, N(a), [_pr(N(x))]))
    elif kinds == 1:
        body.append(_pr(C("Len", N(a))))
    elif kinds == 2:
        body.append(FOR(x, N(a), [_pr(BIN(N(x), "+", g.i(), sp=g.r.random() < 0.5))]))
    else:
        body.append(_pr(C("Pop", edit(N(a)))))
    if g.r.random() < 0.5:
        body.append(_pr(C("Len", N(a))))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K01",))
def v_result_main(g):
    body = list(g.pad())
    v = g.var()
    body.append(LET(v, g.iexpr([])))
    if g.r.random() < 0.6:
        body.append(_pr(N(v)))
    body.append(Ret(C("Ok", Lit_unit())))
    return _prog([_main(*body, ret="Result<(),I64>")], g)


def Lit_unit():
    return model.Lit(())


@_reg(V_FAMS, cls="V", needs=("K02",))
def v_str_basic(g):
    a = g.var()
    t = g.text(escapes=True)
    body = [LET(a, S(t))]
    b = None
    for _ in range(g.r.randint(1, 4)):
        k = g.r.randrange(6)
        if k == 0:
            body.append(_pr(N(a)))
        elif k == 1:
            body.append(_pr(M(N(a), "ByteLen")))
        elif k == 2:
            body.append(ES(C("AssertEq", M(N(a), "ByteLen"), I(len(utf8ref.text(t))))))
        elif k == 3:
            if b is None:
                b = g.var()
                body.append(VAR(b, N(a)))
            body.append(ASG(N(b), S(g.text()), "="))
        elif k == 4:
            body.append(_pr(BIN(N(a), "==", S(g.text()), sp=g.r.random() < 0.5)))
        else:
            body.append(_pr(BIN(N(a), "==", N(a), sp=True)))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K03",))
def v_str_ops(g):
    a, b = g.var(), g.var()
    body = [LET(a, S(g.text())), LET(b, S(g.text()))]
    for _ in range(g.r.randint(1, 5)):
        k = g.r.randrange(9)
        if k == 0:
            body.append(_pr(M(N(a), "ScalarLen")))
        elif k == 1:
            body.append(_pr(M(N(a), "IsEmpty")))
        elif k == 2:
            body.append(_pr(M(N(a), "Concat", S(g.text()))))
        elif k == 3:
            body.append(_pr(BIN(N(a), g.r.choice(OPS6), N(b), sp=g.r.random() < 0.5)))
        elif k == 4:
            body.append(_pr(C("std.text.FromI64", g.i(-9, 99))))
        elif k == 5:
            body.append(_pr(C("std.parse.I64", S(g.r.choice(NUMS)))))
        elif k == 6:
            body.append(_pr(M(N(a), "SliceBytes", g.i(-1, 4), g.i(-1, 5))))
        elif k == 7:
            sc, x = g.var(), g.var()
            body.append(LET(sc, M(N(a), "Scalars")))
            body.append(FOR(x, N(sc), [_pr(N(x))]))
        else:
            body.append(_pr(BIN(S(g.text()), g.r.choice(OPS6), S(g.text()), sp=True)))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K03",))
def v_array_str(g):
    a, x = g.var(), g.var()
    items = [S(g.text()) for _ in range(g.r.randint(0, 3))]
    body = [VAR(a, ARR(*items), t=None if items else "Array<String>")]
    for _ in range(g.r.randint(0, 2)):
        body.append(ES(C("Push", edit(N(a)), S(g.text()))))
    k = g.r.randrange(3)
    if k == 0:
        body.append(FOR(x, N(a), [_pr(N(x))]))
    elif k == 1:
        body.append(_pr(C("Len", N(a))))
    else:
        body.append(FOR(x, N(a), [_pr(M(N(x), "ByteLen"))]))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K04",))
def v_struct(g):
    sn = g.ty()
    nf = g.r.randint(1, 3)
    fl = []
    for _ in range(nf):
        fl.append((g.fld(), g.r.choice(["I64", "I64", "Bool", "String"])))
    args = []
    for _, t in fl:
        args.append(g.i() if t == "I64" else B(g.r.random() < 0.5) if t == "Bool" else S(g.text()))
    s = g.var()
    decls = [STRUCT(sn, fl)]
    body = [VAR(s, C(sn, *args)) if g.r.random() < 0.5 else LET(s, C(sn, *args))]
    mutable = body[0].mut
    for fname, t in fl:
        k = g.r.randrange(3)
        if k == 0:
            body.append(_pr(F(N(s), fname)))
        elif k == 1 and mutable:
            body.append(ASG(F(N(s), fname), g.i() if t == "I64" else B(True) if t == "Bool" else S(g.text())))
        else:
            body.append(ES(C("AssertEq", F(N(s), fname), F(N(s), fname))))
    if g.r.random() < 0.4:
        c = g.var()
        body.append(LET(c, N(s)))
    decls.append(_main(*g.pad(), *body))
    return _prog(decls, g)


@_reg(V_FAMS, cls="V", needs=("K04",))
def v_enum(g):
    en = g.ty()
    nv = g.r.randint(1, 4)
    vs = []
    for _ in range(nv):
        vs.append((g.vnt(), g.r.choice([[], [], ["I64"], ["String"], ["I64", "Bool"]])))
    e, x = g.var(), g.var()
    body = []
    pick = g.r.choice(vs)
    pargs = []
    for t in pick[1]:
        pargs.append(g.i() if t == "I64" else B(True) if t == "Bool" else S(g.text()))
    body.append(LET(e, V(en, pick[0], *pargs, null=not pick[1])))
    if g.r.random() < 0.5:
        a = g.var()
        body.append(LET(a, ARR(V(en, pick[0], *pargs, null=not pick[1]))))
        body.append(FOR(x, N(a), []))
    return _prog([ENUM(en, vs), _main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K05",))
def v_resource(g):
    sn, fn_ = g.ty(), g.fld()
    s = g.var()
    n = g.r.randint(1, 9)
    decls = [STRUCT(sn, [(fn_, "MockResource")])]
    body = []
    mode = g.r.randrange(3)
    if mode == 0:
        body.append(LET(s, C(sn, C("MockOpen", I(n)))))
    elif mode == 1:
        body.append(LET(s, C(sn, C("MockOpen", I(n)))))
        t = g.var()
        body.append(LET(t, Mv(N(s))))
    else:
        r = g.var()
        body.append(LET(r, C("MockOpen", I(n))))
        body.append(ES(C("Inspect", N(r))))
    decls.append(_main(*g.pad(), *body))
    return _prog(decls, g)


@_reg(V_FAMS, cls="V", needs=("K06",))
def v_method(g):
    sn, fl, m_ = g.ty(), g.fld(), g.mth()
    s = g.var()
    mode = g.r.choice(["view", "edit", "move"])
    ret = None
    body = []
    if mode == "view":
        meth = METH(m_, "view", [Ret(F(N("self"), fl))], ret="I64")
        call = _pr(M(N(s), m_))
    elif mode == "edit":
        meth = METH(m_, "edit", [ASG(F(N("self"), fl), g.i())])
        call = ES(M(N(s), m_, prefix="edit"))
    else:
        meth = METH(m_, "move", [])
        call = ES(M(N(s), m_, prefix="move"))
    decls = [STRUCT(sn, [(fl, "I64")]), EXT(sn, meth), UseExt("package." + sn),
             _main(*g.pad(), VAR(s, C(sn, g.i())) if mode == "edit" else LET(s, C(sn, g.i())), call)]
    return _prog(decls, g)


@_reg(V_FAMS, cls="V", needs=("K07",))
def v_match(g):
    en = g.ty()
    names = []
    vs = []
    for _ in range(g.r.randint(2, 4)):
        nm = g.vnt()
        vs.append((nm, g.r.choice([[], ["I64"], ["I64", "Bool"]])))
    e = g.var()
    pick = g.r.choice(vs)
    pargs = [g.i() if t == "I64" else B(g.r.random() < 0.5) for t in pick[1]]
    arms = []
    for nm, pl in vs:
        subs = None
        binds = []
        if pl:
            subs = []
            for t in pl:
                if g.r.random() < 0.5:
                    v = g.var()
                    subs.append(PBind(v))
                    binds.append(v)
                else:
                    subs.append(PWild())
        body = [_pr(N(binds[0]))] if binds and pl[0] == "I64" and isinstance(subs[0], PBind) else []
        arms.append(ARM(PVar(en, nm, subs), body))
    body = [LET(e, V(en, pick[0], *pargs, null=not pick[1])), MATCH(N(e), *arms)]
    return _prog([ENUM(en, vs), _main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K08",))
def v_derive(g):
    sn = g.ty()
    fl = [(g.fld(), g.r.choice(["I64", "Bool", "String"])) for _ in range(g.r.randint(0, 2))]
    args = [g.i() if t == "I64" else B(g.r.random() < 0.5) if t == "Bool" else S(g.text()) for _, t in fl]
    derive = g.r.choice([["Format"], ["PartialEq"], ["Format", "PartialEq"]])
    s, t = g.var(), g.var()
    body = [LET(s, C(sn, *args)), LET(t, C(sn, *args))]
    if "Format" in derive:
        body.append(_pr(N(s)))
        if g.r.random() < 0.5:
            body.append(_pr(S(g.text(), N(s))))
    if "PartialEq" in derive:
        body.append(_pr(BIN(N(s), g.r.choice(["==", "!="]), N(t), sp=True)))
    return _prog([STRUCT(sn, fl, derive=derive), _main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K08",))
def v_holes(g):
    a, b = g.var(), g.var()
    body = [LET(a, g.i()), LET(b, S(g.text()))]
    parts = []
    for _ in range(g.r.randint(1, 3)):
        parts.append(g.text())
        parts.append(g.r.choice([N(a), N(b), BIN(N(a), "+", g.i(), sp=False), g.i(), B(True)]))
    if g.r.random() < 0.5:
        parts.append(g.text())
    body.append(_pr(S(*parts)))
    return _prog([_main(*g.pad(), *body)], g)


@_reg(V_FAMS, cls="V", needs=("K09",))
def v_pub(g):
    sn, fl, fnn = g.ty(), g.fld(), g.fn()
    mode = g.r.randrange(3)
    s, al = g.var(), g.ty()
    if mode == 0:
        decls = [STRUCT(sn, [(fl, "I64")], pub=True),
                 FN(fnn, [("x", "view", sn)], [], ret="()", with_="unrestricted", pub=True),
                 _main(*g.pad(), LET(s, C(sn, g.i())), ES(C(fnn, N(s))))]
    elif mode == 1:
        decls = [STRUCT(sn, [(fl, "I64")]), Use("package." + sn, al),
                 _main(*g.pad(), LET(s, C(sn, g.i())))]
    else:
        decls = [model.Use("std.parse"), _main(*g.pad(), _pr(C("parse.I64", S(g.r.choice(NUMS)))))]
    return _prog(decls, g)


# ---- N: negatives with their repaired positive

def _fin(decls, g, sep, pad):
    """Append an empty Main when the declarations have none."""
    ds = list(decls)
    if not any(isinstance(d, model.Fn) and d.name == "Main" for d in ds):
        ds.append(_main(*pad))
    return Program(ds, sep=sep)


def _n(name, needs, code, sub, fields, **kw):
    return _reg(N_FAMS, cls="N", needs=needs, code=code, sub=sub, fields=fields,
                max_forms=kw.pop("max_forms", PAD_FORMS * 2), **kw)


@_n("n_string_index", ("K02",), "NX-E0102", "not_indexable", "C D P Rel")
def n_string_index(g):
    pad, s, t, i = g.pad(), g.var(), g.text(), g.i(0, 3)
    return lambda pos: [_main(*pad, LET(s, IDX(ARR(I(ord(t[0]) if t else 7)), I(0)) if pos else IDX(S(t), i)))]


@_n("n_noalloc_copy", ("K02",), "NX-E0412", None, "C D")
def n_noalloc_copy(g):
    pad, f, s, t = g.pad(), g.fn(), g.var(), g.var()
    return lambda pos: [FN(f, [(s, "view", "String")],
                           [ES(M(N(s), "ByteLen"))] if pos else [LET(t, N(s))], with_="no_alloc"),
                        _main(*pad)]


@_n("n_noalloc_literal", ("K02",), "NX-E0412", None, "C D P")
def n_noalloc_literal(g):
    pad, f, s, tx = g.pad(), g.fn(), g.var(), g.text()
    k = g.i()
    return lambda pos: [FN(f, [], [LET(s, k if pos else S(tx))], with_="no_alloc"), _main(*pad)]


@_n("n_unknown_string_method", ("K03",), "NX-E1402", "unknown_member", "C D P Rel")
def n_unknown_string_method(g):
    pad, s, t, tx = g.pad(), g.var(), g.var(), g.text()
    bad = g.r.choice(UNKNOWN_METHODS)
    good = g.r.choice(["ByteLen", "ScalarLen", "IsEmpty"])
    return lambda pos: [_main(*pad, LET(s, S(tx)), LET(t, M(N(s), good if pos else bad)))]


@_n("n_order_string_int", ("K03",), "NX-E0102", "type_mismatch", "C D")
def n_order_string_int(g):
    pad, x, y, op, sp = g.pad(), g.text(), g.text(), g.r.choice(OPS6), g.r.random() < 0.5
    k = g.i(0, 9)
    return lambda pos: [_main(*pad, LET("r_x", BIN(S(x), op, S(y) if pos else k, sp=sp)))]


@_n("n_order_int_string", ("K03",), "NX-E0102", "type_mismatch", "C D")
def n_order_int_string(g):
    pad, x, y, op, sp = g.pad(), g.text(), g.text(), g.r.choice(OPS6), g.r.random() < 0.5
    k = g.i(0, 9)
    return lambda pos: [_main(*pad, LET("r_x", BIN(S(x) if pos else k, op, S(y), sp=sp)))]


@_n("n_order_bool", ("K03",), "NX-E0102", "type_mismatch", "C D")
def n_order_bool(g):
    pad, op = g.pad(), g.r.choice(OPS6[:4])
    x, y = g.text(), g.text()
    b1, b2 = B(g.r.random() < 0.5), B(g.r.random() < 0.5)
    return lambda pos: [_main(*pad, LET("r_x", BIN(S(x) if pos else b1, op, S(y) if pos else b2, sp=True)))]


@_n("n_string_plus", ("K03",), "NX-E0102", "type_mismatch", "C D")
def n_string_plus(g):
    pad, x, y = g.pad(), g.text(), g.text()
    return lambda pos: [_main(*pad, LET("r_x", BIN(S(x), "==" if pos else "+", S(y), sp=True)))]


@_n("n_for_string", ("K03",), "NX-E0102", "type_mismatch", "C D P Rel")
def n_for_string(g):
    pad, x, tx = g.pad(), g.var(), g.text()
    return lambda pos: [_main(*pad, FOR(x, ARR(S(tx)) if pos else S(tx), []))]


@_n("n_for_edit", ("K03",), "NX-E0006", None, "C D")
def n_for_edit(g):
    pad, x, tx = g.pad(), g.var(), g.text()
    return lambda pos: [_main(*pad, FOR(x, ARR(S(tx)), [], edit_=not pos))]


@_n("n_graphemes", ("K03",), "NX-E0006", None, "C D")
def n_graphemes(g):
    pad, s, tx = g.pad(), g.var(), g.text()
    bad = g.r.choice(["Graphemes", "View"])
    return lambda pos: [_main(*pad, LET(s, S(tx)), ES(M(N(s), "Scalars" if pos else bad)))]


@_n("n_b01_text", ("K03",), "NX-E1105", "active_borrow_conflict", "C")
def n_b01_text(g):
    pad, a, x, t1, t2 = g.pad(), g.var(), g.var(), g.text(), g.text()
    return lambda pos: [_main(*pad, VAR(a, ARR(S(t1))),
                              FOR(x, ARR(S(t1)) if pos else N(a), [ASG(IDX(N(a), I(0)), S(t2))]))]


@_n("n_unknown_field", ("K04",), "NX-E1402", "unknown_member", "C D P Rel")
def n_unknown_field(g):
    pad, sn, x, y, s = g.pad(), g.ty(), g.fld(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        _main(*pad, LET(s, C(sn, k)), ES(F(N(s), x if pos else y)))]


@_n("n_dup_field", ("K04",), "NX-E0104", None, "C D P Rel")
def n_dup_field(g):
    pad, sn, x, y = g.pad(), g.ty(), g.fld(), g.fld()
    return lambda pos: [STRUCT(sn, [(x, "I64"), (y if pos else x, "I64")]), _main(*pad)]


@_n("n_dup_variant", ("K04",), "NX-E0104", None, "C D P Rel")
def n_dup_variant(g):
    pad, en, a, b = g.pad(), g.ty(), g.vnt(), g.vnt()
    return lambda pos: [ENUM(en, [(a, []), (b if pos else a, [])]), _main(*pad)]


@_n("n_missing_arg", ("K04",), "NX-E0111", "missing_argument", "C D")
def n_missing_arg(g):
    pad, sn, x, s = g.pad(), g.ty(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]), _main(*pad, LET(s, C(sn, k) if pos else C(sn)))]


@_n("n_unknown_label", ("K04",), "NX-E0111", "unknown_argument_label", "C D")
def n_unknown_label(g):
    pad, sn, x, z, s = g.pad(), g.ty(), g.fld(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]), _main(*pad, LET(s, C(sn, lab(x if pos else z, k))))]


@_n("n_assign_let_field", ("K04",), "NX-E0105", None, "C")
def n_assign_let_field(g):
    pad, sn, x, s = g.pad(), g.ty(), g.fld(), g.var()
    k1, k2 = g.i(), g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        _main(*pad, (VAR if pos else LET)(s, C(sn, k1)), ASG(F(N(s), x), k2))]


@_n("n_recursive_layout", ("K04",), "NX-E1323", "recursive_value_layout", "C P Rel")
def n_recursive_layout(g):
    pad, sn, x = g.pad(), g.ty(), g.fld()
    return lambda pos: [STRUCT(sn, [(x, "I64" if pos else sn)]), _main(*pad)]


@_n("n_unknown_variant", ("K04",), "NX-E1402", "unknown_member", "C D P Rel")
def n_unknown_variant(g):
    pad, en, a, b, e = g.pad(), g.ty(), g.vnt(), g.vnt(), g.var()
    return lambda pos: [ENUM(en, [(a, [])]), _main(*pad, LET(e, V(en, a if pos else b, null=True)))]


@_n("n_nominal_mismatch", ("K04",), "NX-E0102", "type_mismatch", "C D")
def n_nominal_mismatch(g):
    pad, a, b, x = g.pad(), g.ty(), g.ty(), g.var()
    return lambda pos: [STRUCT(a, []), STRUCT(b, []), _main(*pad, LET(x, C(a if pos else b), t=a))]


@_n("n_empty_enum", ("K04",), "NX-E0006", None, "C D")
def n_empty_enum(g):
    pad, en, a = g.pad(), g.ty(), g.vnt()
    return lambda pos: [ENUM(en, [(a, [])] if pos else []), _main(*pad)]


@_n("n_result_layout", ("K04",), "NX-E1323", "recursive_value_layout", "C P Rel")
def n_result_layout(g):
    pad, sn, x = g.pad(), g.ty(), g.fld()
    return lambda pos: [STRUCT(sn, [(x, "Result<I64,I64>" if pos else "Result<%s,I64>" % sn)]), _main(*pad)]


@_n("n_non_callable", ("K04",), "NX-E0111", "non_callable_target", "C D")
def n_non_callable(g):
    pad, en, a = g.pad(), g.ty(), g.vnt()
    return lambda pos: [ENUM(en, [(a, [])]), _main(*pad, ES(V(en, a, null=True, called=not pos)))]


@_n("n_drop_field", ("K04",), "NX-E0006", None, "C D")
def n_drop_field(g):
    pad, sn, x, p = g.pad(), g.ty(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        _main(*pad, LET(p, C(sn, k)), ES(C("Drop", N(p) if pos else F(N(p), x))))]


@_n("n_mutual_recursion", ("K04",), "NX-E1323", "recursive_value_layout", "C P Rel")
def n_mutual_recursion(g):
    pad, a, b, x, y = g.pad(), g.ty(), g.ty(), g.fld(), g.fld()
    return lambda pos: [STRUCT(a, [(x, b)]), STRUCT(b, [(y, "I64" if pos else a)]), _main(*pad)]


@_n("n_for_nominal", ("K04",), "NX-E0102", "type_mismatch", "C D P Rel")
def n_for_nominal(g):
    pad, sn, x = g.pad(), g.ty(), g.var()
    return lambda pos: [STRUCT(sn, []), _main(*pad, FOR(x, ARR(C(sn)) if pos else C(sn), []))]


@_n("n_b01_struct", ("K04",), "NX-E1105", "active_borrow_conflict", "C")
def n_b01_struct(g):
    pad, sn, x, a, it = g.pad(), g.ty(), g.fld(), g.var(), g.var()
    k1, k2 = g.i(), g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        _main(*pad, VAR(a, ARR(C(sn, k1))),
                              FOR(it, ARR(C(sn, k1)) if pos else N(a), [ASG(IDX(N(a), I(0)), C(sn, k2))]))]


@_n("n_b01_enum", ("K04",), "NX-E1105", "active_borrow_conflict", "C")
def n_b01_enum(g):
    pad, en, v, a, it = g.pad(), g.ty(), g.vnt(), g.var(), g.var()
    k1, k2 = g.i(), g.i()
    return lambda pos: [ENUM(en, [(v, ["I64"])]),
                        _main(*pad, VAR(a, ARR(V(en, v, k1))),
                              FOR(it, ARR(V(en, v, k1)) if pos else N(a), [ASG(IDX(N(a), I(0)), V(en, v, k2))]))]


@_n("n_copy_resource", ("K05",), "NX-E0201", None, "C")
def n_copy_resource(g):
    pad, sn, r, s, t = g.pad(), g.ty(), g.fld(), g.var(), g.var()
    k = g.i(1, 9)
    return lambda pos: [STRUCT(sn, [(r, "MockResource")]),
                        _main(*pad, LET(s, C(sn, C("MockOpen", k))), LET(t, Mv(N(s)) if pos else N(s)))]


@_n("n_partial_move", ("K05",), "NX-E0006", None, "C D")
def n_partial_move(g):
    pad, sn, x, s, t = g.pad(), g.ty(), g.fld(), g.var(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        _main(*pad, LET(s, C(sn, k)), LET(t, F(N(s), x) if pos else Mv(F(N(s), x))))]


@_n("n_enum_resource_copy", ("K05",), "NX-E0201", None, "C")
def n_enum_resource_copy(g):
    pad, en, a, b, e, f_ = g.pad(), g.ty(), g.vnt(), g.vnt(), g.var(), g.var()
    k = g.i(1, 9)
    return lambda pos: [ENUM(en, [(a, []), (b, ["MockResource"])]),
                        _main(*pad, LET(e, V(en, a, null=True)), LET(f_, Mv(N(e)) if pos else N(e)))]


@_n("n_permission_escalation", ("K05",), "NX-E1105", "permission_escalation", "C")
def n_permission_escalation(g):
    pad, sn, x, f, s = g.pad(), g.ty(), g.fld(), g.fn(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        FN(f, [(s, "edit" if pos else "view", sn)], [ASG(F(N(s), x), k)]), _main(*pad)]


@_n("n_overlapping_borrows", ("K05",), "NX-E1105", "overlapping_mutable_borrows", "C")
def n_overlapping_borrows(g):
    pad, sn, x, y, f, a, b, s, t = g.pad(), g.ty(), g.fld(), g.fld(), g.fn(), g.var(), g.var(), g.var(), g.var()
    k1, k2, k3, k4 = g.i(), g.i(), g.i(), g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64"), (y, "I64")]), FN(f, [(a, "edit", "I64"), (b, "edit", "I64")], []),
                        _main(*pad, *([VAR(s, C(sn, k1, k2)), VAR(t, C(sn, k3, k4)),
                                       ES(C(f, edit(F(N(s), x)), edit(F(N(t), y))))] if pos else
                                      [VAR(s, C(sn, k1, k2)),
                                       ES(C(f, edit(F(N(s), x)), edit(F(N(s), y))))]))]


@_n("n_use_after_drop", ("K05",), "NX-E0202", "definitely_consumed", "C")
def n_use_after_drop(g):
    pad, sn, s, t = g.pad(), g.ty(), g.var(), g.var()
    return lambda pos: [STRUCT(sn, []),
                        _main(*pad, LET(s, C(sn)), *([] if pos else [ES(C("Drop", N(s)))]), LET(t, N(s)))]


@_n("n_array_resource", ("K05",), "NX-E0006", None, "C D")
def n_array_resource(g):
    pad, sn, r, a = g.pad(), g.ty(), g.fld(), g.var()
    return lambda pos: [STRUCT(sn, [(r, "I64" if pos else "MockResource")]),
                        _main(*pad, LET(a, ARR(), t="Array<%s>" % sn))]


@_n("n_missing_edit_prefix", ("K06",), "NX-E1107", None, "C D P Rel")
def n_missing_edit_prefix(g):
    pad, sn, m_, s = g.pad(), g.ty(), g.mth(), g.var()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m_, "edit")), UseExt("package." + sn),
                        _main(*pad, VAR(s, C(sn)), ES(M(N(s), m_, prefix="edit" if pos else None)))]


@_n("n_no_use_extension", ("K06",), "NX-E1315", None, "C")
def n_no_use_extension(g):
    pad, sn, m_, s = g.pad(), g.ty(), g.mth(), g.var()
    return lambda pos: [*([STRUCT(sn, []), EXT(sn, METH(m_, "view"))] + ([UseExt("package." + sn)] if pos else [])),
                        _main(*pad, LET(s, C(sn)), ES(M(N(s), m_)))]


@_n("n_duplicate_method", ("K06",), "NX-E1322", None, "C")
def n_duplicate_method(g):
    pad, sn, m1, m2 = g.pad(), g.ty(), g.mth(), g.mth()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m1, "view"), METH(m2 if pos else m1, "view")), _main(*pad)]


@_n("n_self_view_write", ("K06",), "NX-E1105", "permission_escalation", "C")
def n_self_view_write(g):
    pad, sn, x, m_ = g.pad(), g.ty(), g.fld(), g.mth()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")]),
                        EXT(sn, METH(m_, "edit" if pos else "view", [ASG(F(N("self"), x), k)])), _main(*pad)]


@_n("n_temp_edit", ("K06",), "NX-E1107", None, "C D P")
def n_temp_edit(g):
    pad, sn, m_ = g.pad(), g.ty(), g.mth()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m_, "view" if pos else "edit")), UseExt("package." + sn),
                        _main(*pad, ES(M(C(sn), m_)))]


@_n("n_temp_move", ("K06",), "NX-E1107", None, "C D P")
def n_temp_move(g):
    pad, sn, m_ = g.pad(), g.ty(), g.mth()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m_, "view" if pos else "move")), UseExt("package." + sn),
                        _main(*pad, ES(M(C(sn), m_)))]


@_n("n_edit_non_place", ("K06",), "NX-E0002", None, "C P T")
def n_edit_non_place(g):
    pad, sn, m_, s = g.pad(), g.ty(), g.mth(), g.var()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m_, "edit")), UseExt("package." + sn),
                        _main(*pad, *([VAR(s, C(sn)), ES(M(N(s), m_, prefix="edit"))] if pos
                                      else [ES(M(C(sn), m_, prefix="edit"))]))]


@_n("n_move_non_place", ("K06",), "NX-E0002", None, "C P T")
def n_move_non_place(g):
    pad, sn, m_, s = g.pad(), g.ty(), g.mth(), g.var()
    return lambda pos: [STRUCT(sn, []), EXT(sn, METH(m_, "move")), UseExt("package." + sn),
                        _main(*pad, *([LET(s, C(sn)), ES(M(N(s), m_, prefix="move"))] if pos
                                      else [ES(M(C(sn), m_, prefix="move"))]))]


@_n("n_pattern_arity", ("K07",), "NX-E1325", "constructor_arity", "C D")
def n_pattern_arity(g):
    pad, en, a, e, x, y = g.pad(), g.ty(), g.vnt(), g.var(), g.var(), g.var()
    k1, k2 = g.i(), g.i()
    return lambda pos: [ENUM(en, [(a, ["I64", "I64"])]),
                        _main(*pad, LET(e, V(en, a, k1, k2)),
                              MATCH(N(e), ARM(PVar(en, a, [PBind(x)] + ([PBind(y)] if pos else [])))))]


@_n("n_non_exhaustive", ("K07",), "NX-E1304", "non_exhaustive", "C D")
def n_non_exhaustive(g):
    pad, en, a, b, e = g.pad(), g.ty(), g.vnt(), g.vnt(), g.var()
    return lambda pos: [ENUM(en, [(a, []), (b, [])]),
                        _main(*pad, MATCH(V(en, a, null=True),
                                          *([ARM(PVar(en, a))] + ([ARM(PVar(en, b))] if pos else []))))]


@_n("n_non_exhaustive_witness", ("K07",), "NX-E1304", "non_exhaustive", "C D")
def n_non_exhaustive_witness(g):
    pad, en, a, b = g.pad(), g.ty(), g.vnt(), g.vnt()
    return lambda pos: [ENUM(en, [(a, []), (b, ["I64", "Bool"])]),
                        _main(*pad, MATCH(V(en, a, null=True),
                                          *([ARM(PVar(en, a))] +
                                            ([ARM(PVar(en, b, [PWild(), PWild()]))] if pos else []))))]


@_n("n_print_no_format", ("K08",), "NX-E1403", "missing_format", "C D P Rel")
def n_print_no_format(g):
    pad, sn, x, s = g.pad(), g.ty(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")], derive=["Format"] if pos else []),
                        _main(*pad, LET(s, C(sn, k)), _pr(N(s)))]


@_n("n_eq_no_partial_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_eq_no_partial_eq(g):
    pad, sn, x, s = g.pad(), g.ty(), g.fld(), g.var()
    k = g.i()
    return lambda pos: [STRUCT(sn, [(x, "I64")], derive=["PartialEq"] if pos else []),
                        _main(*pad, LET(s, C(sn, k)), ES(C("AssertEq", BIN(N(s), "==", N(s), sp=True), B(True))))]


@_n("n_format_resource_field", ("K08",), "NX-E1403", "missing_format", "C D P Rel")
def n_format_resource_field(g):
    pad, sn, r = g.pad(), g.ty(), g.fld()
    return lambda pos: [STRUCT(sn, [(r, "I64" if pos else "MockResource")], derive=["Format"]), _main(*pad)]


@_n("n_duplicate_derive", ("K08",), "NX-E1404", "duplicate_derive", "C D P Rel")
def n_duplicate_derive(g):
    pad, sn = g.pad(), g.ty()
    cap = g.r.choice(["Format", "PartialEq"])
    return lambda pos: [STRUCT(sn, [], derive=[cap] if pos else [cap, cap]), _main(*pad)]


@_n("n_derive_on_fn", ("K08",), "NX-E1404", "invalid_derive_target", "C D P Rel")
def n_derive_on_fn(g):
    pad, nm = g.pad(), g.ty()
    return lambda pos: [STRUCT(nm, [], derive=["Format"]) if pos else FN(nm, [], [], derive=["Format"]),
                        _main(*pad)]


@_n("n_interp_no_format", ("K08",), "NX-E1403", "missing_format", "C D P Rel")
def n_interp_no_format(g):
    pad, sn, s, t = g.pad(), g.ty(), g.var(), g.var()
    return lambda pos: [STRUCT(sn, [], derive=["Format"] if pos else []),
                        _main(*pad, LET(s, C(sn)), LET(t, S(N(s))))]


@_n("n_partial_eq_resource", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_partial_eq_resource(g):
    pad, sn, r = g.pad(), g.ty(), g.fld()
    return lambda pos: [STRUCT(sn, [(r, "I64" if pos else "MockResource")], derive=["PartialEq"]), _main(*pad)]


@_n("n_derive_hash", ("K08",), "NX-E0006", None, "C D")
def n_derive_hash(g):
    pad, sn = g.pad(), g.ty()
    bad = g.r.choice(["Hash", "Ord", "Clone"])
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq" if pos else bad]), _main(*pad)]


@_n("n_nominal_ne_no_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_nominal_ne_no_eq(g):
    pad, sn, op = g.pad(), g.ty(), g.r.choice(["==", "!="])
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"] if pos else []),
                        _main(*pad, LET("r_s", C(sn)), LET("r_x", BIN(N("r_s"), op, N("r_s"), sp=True)))]


@_n("n_array_ne_no_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_array_ne_no_eq(g):
    pad, sn, op = g.pad(), g.ty(), g.r.choice(["==", "!="])
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"] if pos else []),
                        _main(*pad, LET("r_a", ARR(), t="Array<%s>" % sn),
                              LET("r_x", BIN(N("r_a"), op, N("r_a"), sp=True)))]


@_n("n_nominal_types_differ", ("K08",), "NX-E0102", "type_mismatch", "C D")
def n_nominal_types_differ(g):
    pad, a, b, op = g.pad(), g.ty(), g.ty(), g.r.choice(["==", "!="])
    return lambda pos: [STRUCT(a, [], derive=["PartialEq"]), STRUCT(b, [], derive=["PartialEq"]),
                        _main(*pad, LET("r_x", BIN(C(a), op, C(a if pos else b), sp=True)))]


@_n("n_nominal_order", ("K08",), "NX-E0102", "type_mismatch", "C D")
def n_nominal_order(g):
    pad, sn, op = g.pad(), g.ty(), g.r.choice(OPS6[:4])
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"]),
                        _main(*pad, LET("r_s", C(sn)), LET("r_x", BIN(N("r_s"), "==" if pos else op, N("r_s"), sp=True)))]


@_n("n_option_ne_no_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_option_ne_no_eq(g):
    pad, sn = g.pad(), g.ty()
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"] if pos else []),
                        _main(*pad, LET("r_a", C("Some", C(sn))), LET("r_x", BIN(N("r_a"), "!=", N("r_a"), sp=True)))]


@_n("n_result_ne_no_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_result_ne_no_eq(g):
    pad, sn = g.pad(), g.ty()
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"] if pos else []),
                        _main(*pad, LET("r_a", C("Ok", C(sn)), t="Result<%s,I64>" % sn),
                              LET("r_x", BIN(N("r_a"), "!=", N("r_a"), sp=True)))]


@_n("n_enum_payload_no_eq", ("K08",), "NX-E1403", "missing_partial_eq", "C D P Rel")
def n_enum_payload_no_eq(g):
    pad, sn, en, a, b = g.pad(), g.ty(), g.ty(), g.vnt(), g.vnt()
    return lambda pos: [STRUCT(sn, [], derive=["PartialEq"] if pos else []),
                        ENUM(en, [(a, []), (b, [sn])], derive=["PartialEq"]), _main(*pad)]


@_n("n_format_hole_no_format", ("K08",), "NX-E1403", "missing_format", "C D P Rel")
def n_format_hole_no_format(g):
    pad, sn, s, t = g.pad(), g.ty(), g.var(), g.var()
    t1, t2 = g.text(), g.text()
    return lambda pos: [STRUCT(sn, [], derive=["Format"] if pos else []),
                        _main(*pad, LET(s, C(sn)), LET(t, S(t1, N(s), t2)))]


@_n("n_missing_signature_type", ("K09",), "NX-E1310", "missing_signature_type", "C P Rel")
def n_missing_signature_type(g):
    pad, f = g.pad(), g.fn()
    return lambda pos: [FN(f, [], [], ret="()" if pos else None, with_="unrestricted", pub=True), _main(*pad)]


@_n("n_missing_effect_bound", ("K09",), "NX-E1310", "missing_effect_bound", "C P Rel")
def n_missing_effect_bound(g):
    pad, f = g.pad(), g.fn()
    return lambda pos: [FN(f, [], [], ret="()", with_="unrestricted" if pos else None, pub=True), _main(*pad)]


@_n("n_unexported_type", ("K09",), "NX-E1310", "unexported_type", "C P Rel")
def n_unexported_type(g):
    pad, sn, f, x = g.pad(), g.ty(), g.fn(), g.var()
    return lambda pos: [STRUCT(sn, [], pub=pos), FN(f, [(x, "view", sn)], [], ret="()", with_="unrestricted", pub=True),
                        _main(*pad)]


@_n("n_private_reexport", ("K09",), "NX-E1308", "private_reexport", "C P Rel")
def n_private_reexport(g):
    pad, sn, al = g.pad(), g.ty(), g.ty()
    return lambda pos: [STRUCT(sn, [], pub=pos), Use("package." + sn, al, pub=True), _main(*pad)]


@_n("n_module_decl", ("K09",), "NX-E0006", None, "C D")
def n_module_decl(g):
    pad, sn = g.pad(), g.ty()
    nm = g.r.choice(["extra", "util", "net", "core"])
    return lambda pos: [STRUCT(sn, []) if pos else RawDecl("mod %s;" % nm, "mod"), _main(*pad)]


@_n("n_alias_collision", ("K09",), "NX-E1309", "alias_collision", "C P Rel")
def n_alias_collision(g):
    pad, a, b, c = g.pad(), g.ty(), g.ty(), g.ty()
    return lambda pos: [STRUCT(a, []), STRUCT(b, []), Use("package." + a, c if pos else b), _main(*pad)]


@_n("n_unexported_nominal_payload", ("K09",), "NX-E1310", "unexported_type", "C P Rel")
def n_unexported_nominal_payload(g):
    pad, sn, en, a = g.pad(), g.ty(), g.ty(), g.vnt()
    return lambda pos: [STRUCT(sn, [], pub=pos), ENUM(en, [(a, [sn])], pub=True), _main(*pad)]


@_n("n_trait_decl", ("K04",), "NX-E0006", None, "C D")
def n_trait_decl(g):
    pad, sn = g.pad(), g.ty()
    return lambda pos: [STRUCT(sn, []) if pos else RawDecl("trait %s{}" % sn, "trait"), _main(*pad)]


# ---- L: grammar of the Hole (one subcause per source)

WS = ["", " ", "\t", " \t ", "  "]


def _l(name, sub):
    return _reg(L_FAMS, cls="L", needs=("K01",), code="NX-E1401", sub=sub, fields="C D P Rel T",
                max_forms=PAD_FORMS * 5, verbs=("check", "run"))


def _lit_rep(g, pre, body, post, shape, sub):
    pads = g.pad()
    ctx = g.r.choice(["let", "let", "print"])
    v = g.var()
    head = ('let %s="' % v) if ctx == "let" else 'Print("'
    tail = '";' if ctx == "let" else '");'
    return {"pads": [to_repr(p) for p in pads], "ctx": ctx, "var": v, "head": head + pre,
            "body": body, "tail": post + tail, "positive_text": pre + post,
            "shape": shape + "|" + ctx + "|" + sha_hex(structural_program(Program(pads))), "sub": sub}


@_l("l_empty", "empty_interpolation")
def l_empty(g):
    ws = g.r.choice(WS)
    pre = g.r.choice(TEXTS[:20])
    post = g.r.choice(TEXTS[:12])
    return _lit_rep(g, pre, "{" + ws + "}", post, "empty|%s|%s|%s" % (bool(pre), bool(post), ws), "empty_interpolation")


@_l("l_unescaped_close", "unescaped_closing_brace")
def l_unescaped_close(g):
    pre = g.r.choice(TEXTS[:20])
    post = g.r.choice(TEXTS[:12])
    return _lit_rep(g, pre, "}", post, "close|%s|%s" % (bool(pre), bool(post)), "unescaped_closing_brace")


@_l("l_unterminated", "unterminated_interpolation")
def l_unterminated(g):
    pre = g.r.choice(TEXTS[:20])
    digits = str(g.r.choice([1, 7, 42, 100, 0, 305]))
    return _lit_rep(g, pre, "{" + digits, "", "unterm|%s|%d" % (bool(pre), len(digits)), "unterminated_interpolation")


@_l("l_incomplete_open", "incomplete_interpolation_expression")
def l_incomplete_open(g):
    pre = g.r.choice(TEXTS[:20])
    return _lit_rep(g, pre, "{", "", "incopen|%s" % bool(pre), "incomplete_interpolation_expression")


@_l("l_incomplete_expr", "incomplete_interpolation_expression")
def l_incomplete_expr(g):
    pre = g.r.choice(TEXTS[:20])
    digits = str(g.r.choice([1, 2, 9, 77]))
    return _lit_rep(g, pre, "{" + digits + " + ", "", "incexpr|%s|%d" % (bool(pre), len(digits)),
                    "incomplete_interpolation_expression")


@_l("l_raw_break", "raw_line_break_in_interpolation")
def l_raw_break(g):
    pre = g.r.choice(TEXTS[:20])
    brk = g.r.choice(["\r\n", "\r", "\n"])
    digits = str(g.r.choice([1, 5, 12]))
    return _lit_rep(g, pre, "{" + digits + brk + "}", "", "brk|%s|%d" % (bool(pre), len(brk)),
                    "raw_line_break_in_interpolation")


@_l("l_comment", "comment_in_interpolation")
def l_comment(g):
    pre = g.r.choice(TEXTS[:20])
    digits = str(g.r.choice([1, 5, 12]))
    return _lit_rep(g, pre, "{" + digits + "//}", "", "cmt|%s|%d" % (bool(pre), len(digits)),
                    "comment_in_interpolation")


# ---- B: one pending construction (K 9.1)

def _b(name, target, uses=(), verbs=("check", "run")):
    return _reg(B_FAMS, cls="B", needs=(), target=target, uses=tuple(uses), fields="C D",
                max_forms=PAD_FORMS * 6, verbs=verbs)


@_b("b_k02", "K02")
def b_k02(g):
    s, a, t = g.var(), g.var(), g.text(escapes=True)
    k = g.r.randrange(4)
    if k == 0:
        body = [LET(s, S(t))]
    elif k == 1:
        body = [_pr(S(t))]
    elif k == 2:
        body = [LET(a, S(t)), LET(s, M(N(a), "ByteLen")), _pr(N(s))]
    else:
        body = [LET(a, S(t)), _pr(BIN(N(a), "==", S(g.text()), sp=True))]
    return _prog([_main(*g.pad(), *body)], g)


@_b("b_k03", "K03", uses=("K02",))
def b_k03(g):
    a, b, s = g.var(), g.var(), g.var()
    k = g.r.randrange(8)
    base = [LET(a, S(g.text()))]
    if k == 0:
        body = base + [LET(s, M(N(a), g.r.choice(["IsEmpty", "ScalarLen"])))]
    elif k == 1:
        body = base + [LET(s, M(N(a), "Concat", S(g.text())))]
    elif k == 2:
        body = base + [LET(s, M(N(a), "SliceBytes", g.i(0, 3), g.i(0, 4)))]
    elif k == 3:
        body = base + [LET(s, M(N(a), "Scalars"))]
    elif k == 4:
        body = [_pr(C("std.parse.I64", S(g.r.choice(NUMS))))]
    elif k == 5:
        body = base + [LET(b, S(g.text())), LET(s, BIN(N(a), g.r.choice(["!=", "<", "<=", ">", ">="]), N(b), sp=True))]
    elif k == 6:
        body = [LET(a, ARR(), t="Array<String>")]
    else:
        body = [FOR(a, ARR(S(g.text())), [])]
    return _prog([_main(*g.pad(), *body)], g)


@_b("b_k04", "K04")
def b_k04(g):
    sn, en, x, s, v = g.ty(), g.ty(), g.fld(), g.var(), g.var()
    k = g.r.randrange(4)
    if k == 0:
        decls = [STRUCT(sn, [(x, "I64")])]
    elif k == 1:
        decls = [ENUM(en, [(g.vnt(), []), (g.vnt(), ["I64"])])]
    elif k == 2:
        decls = [STRUCT(sn, [(x, "I64")]), _main(*g.pad(), LET(s, C(sn, g.i())), LET(v, F(N(s), x)))]
    else:
        decls = [STRUCT(sn, []), _main(*g.pad(), LET(s, ARR(C(sn))))]
    return _prog(decls, g)


@_b("b_k05", "K05", uses=("K04",))
def b_k05(g):
    sn, r, s = g.ty(), g.fld(), g.var()
    decls = [STRUCT(sn, [(r, "MockResource")]), _main(*g.pad(), LET(s, C(sn, C("MockOpen", g.i(1, 9)))))]
    return _prog(decls, g)


@_b("b_k06", "K06", uses=("K04",))
def b_k06(g):
    sn, m_, s = g.ty(), g.mth(), g.var()
    decls = [STRUCT(sn, []), EXT(sn, METH(m_, "view")), UseExt("package." + sn),
             _main(*g.pad(), LET(s, C(sn)), ES(M(N(s), m_)))]
    return _prog(decls, g)


@_b("b_k07", "K07", uses=("K04",))
def b_k07(g):
    en, a, b, e = g.ty(), g.vnt(), g.vnt(), g.var()
    decls = [ENUM(en, [(a, []), (b, [])]),
             _main(*g.pad(), LET(e, V(en, a, null=True)), MATCH(N(e), ARM(PVar(en, a)), ARM(PVar(en, b))))]
    return _prog(decls, g)


@_b("b_k08", "K08", uses=("K02",))
def b_k08(g):
    s = g.var()
    parts = [g.text()]
    for _ in range(g.r.randint(1, 2)):
        parts.append(g.i(0, 9))
        parts.append(g.text())
    return _prog([_main(*g.pad(), LET(s, S(*parts)))], g)


@_b("b_k08_derive", "K08", uses=("K04",))
def b_k08_derive(g):
    sn = g.ty()
    return _prog([STRUCT(sn, [], derive=[g.r.choice(["Format", "PartialEq"])]), _main(*g.pad())], g)


@_b("b_k09", "K09")
def b_k09(g):
    f = g.fn()
    return _prog([FN(f, [], [], ret="()", with_="unrestricted", pub=True), _main(*g.pad())], g)


@_b("b_k09_import", "K09")
def b_k09_import(g):
    return _prog([Use(g.r.choice(["std.parse", "std.text"])), _main(*g.pad())], g)


@_b("b_k09_alias", "K09", uses=("K04",))
def b_k09_alias(g):
    sn, al = g.ty(), g.ty()
    return _prog([STRUCT(sn, []), Use("package." + sn, al), _main(*g.pad())], g)


@_b("b_k10", "K10", verbs=("check",))
def b_k10(g):
    return _prog([_main(*g.pad(), ES(TRY(C("std.console.ReadLine"))), Ret(C("Ok", Lit_unit())),
                        ret="Result<(),I64>")], g)


# ------------------------------------------------------------------ case construction

def families_for(cls: str, closed) -> list:
    closed = set(closed)
    if cls == "V":
        return [f for f in V_FAMS if set(f.needs) <= closed]
    if cls == "N":
        return [f for f in N_FAMS if set(f.needs) <= closed]
    if cls == "L":
        return list(L_FAMS)
    if cls == "B":
        return [f for f in B_FAMS if f.target not in closed and set(f.uses) <= closed]
    raise KeyError(cls)


def _rng(seed, fam, k):
    return random.Random("%s/%d/%s/%d" % (SEED_LABEL, seed, fam.name, k))


def build_reps(fam: Fam, seed: int, k: int, closed) -> list:
    """Reps (main first, then positive) of one attempt; each is a dict with ``cls``."""
    g = Gen(_rng(seed, fam, k))
    closed = sorted(closed)
    base = {"family": fam.name, "closed": closed, "verbs": list(fam.verbs), "fields": fam.fields}
    if fam.cls == "V":
        prog = fam.build(g)
        return [dict(base, cls="V", program=to_repr(prog), fields="")]
    if fam.cls == "N":
        sep = g.sep()
        build = fam.build(g)
        neg = Program(list(build(False)), sep=sep)
        pos = Program(list(build(True)), sep=sep)
        return [dict(base, cls="N", program=to_repr(neg)), dict(base, cls="P", program=to_repr(pos), fields="")]
    if fam.cls == "L":
        lit = fam.build(g)
        return [dict(base, cls="L", lit=lit),
                dict(base, cls="P", program=to_repr(lit_positive_program(lit)), fields="")]
    if fam.cls == "B":
        prog = fam.build(g)
        return [dict(base, cls="B", program=to_repr(prog))]
    raise KeyError(fam.cls)


def shape_of(rep) -> str:
    if rep["cls"] == "L":
        return sha_hex("L|" + rep["lit"]["shape"])
    return sha_hex(structural_program(from_repr(rep["program"])))


def check_main(fam: Fam, rep: dict, res: dict):
    """Return None when the derived status matches the family, else the discard reason."""
    st = res["status"]
    if fam.cls == "V":
        return None if st == "ok" else st
    if fam.cls == "N":
        if st != "diag" or len(res["diags"]) != 1:
            return "n_" + st
        d = res["diags"][0]
        if d["code"] != fam.code or d["subcause"] != fam.sub:
            return "n_code_mismatch"
        return None
    if fam.cls == "L":
        return None if st == "syntax" and res["scope"] == fam.sub else "l_" + st
    if fam.cls == "B":
        if st != "barrier" or res["scope"] != fam.target:
            return "b_" + st
        return None
    return "class"


def case_records(fam: Fam, seed: int, k: int, closed, profile: str):
    """Build the records of one attempt or return (None, reason)."""
    reps = build_reps(fam, seed, k, closed)
    main = reps[0]
    res = expect(main, profile)
    why = check_main(fam, main, res)
    if why is not None:
        return None, why
    out = [(main, res)]
    for rep in reps[1:]:
        r2 = expect(rep, profile)
        if r2["status"] == "ok" or r2["status"] == "barrier":
            out.append((rep, r2))
        else:
            return None, "positive_" + r2["status"]
    return out, None


def make_record(rep, res, seed, index, role, phase, profile, fclass, pair=None):
    exp = res["expected"]
    rec = {"seed": seed, "index": index, "role": role, "cls": rep["cls"], "family": rep["family"],
           "fclass": fclass,
           "phase": phase, "file": FILE, "profile_id": profile,
           "source_hex": res["source"].encode("utf-8").hex(),
           "verbs": [v for v in ("check", "run") if v in exp], "expected": exp,
           "expected_sha256": fio.sha256_text(fio.dumps(exp)), "repr": rep,
           "distinct": sha_hex(res["source"]), "structural": shape_of(rep)}
    if pair is not None:
        rec["pair"] = pair
    return rec


# ------------------------------------------------------------------ seed corpus

INVALID_PATTERN = ["N", "N", "N", "L", "N", "N", "B", "N", "N", "L", "N", "B", "N", "N", "L", "N", "N", "B", "N", "N"]
MAX_RETRY = 40


def _stats():
    return {"attempts": 0, "discards": {}, "distinct": 0, "structural": set(), "positives": 0}


def _discard(st, why):
    st["discards"][why] = st["discards"].get(why, 0) + 1


def generate_seed(seed: int, phase: str, complete, valid: int, invalid: int, profile: str = PROFILE):
    """Records of one seed: ``valid`` V cases and ``invalid`` N/L/B cases (+ their positives)."""
    closed = barrier.validate_manifest(phase, complete)
    if profile != PROFILE:
        raise barrier.ManifestError("generator supports only profile %s (test profiles belong to Q)" % PROFILE)
    records = []
    stats = {"V": _stats(), "N": _stats(), "L": _stats(), "B": _stats()}
    fams = {c: families_for(c, closed) for c in "VNLB"}
    fam_stats = {}
    seen = set()

    def one(cls, fam, counter, role_index):
        for _ in range(MAX_RETRY):
            k = counter.get(fam.name, 0)
            counter[fam.name] = k + 1
            stats[cls]["attempts"] += 1
            got, why = case_records(fam, seed, k, closed, profile)
            if got is None:
                _discard(stats[cls], why)
                continue
            main_rep, main_res = got[0]
            h = sha_hex(main_res["source"])
            if h in seen:
                _discard(stats[cls], "duplicate")
                continue
            seen.add(h)
            idx = role_index
            rec = make_record(main_rep, main_res, seed, idx, "main", phase, profile, cls)
            records.append(rec)
            stats[cls]["distinct"] += 1
            stats[cls]["structural"].add(rec["structural"])
            fs = fam_stats.setdefault(fam.name, {"cls": cls, "distinct": 0, "structural": set(),
                                                 "max_forms": fam.max_forms})
            fs["distinct"] += 1
            fs["structural"].add(rec["structural"])
            for rep, res in got[1:]:
                records.append(make_record(rep, res, seed, idx, "positive", phase, profile, cls, pair=idx))
                stats[cls]["positives"] += 1
            return True
        return False

    counters = {}
    for i in range(valid):
        if fams["V"]:
            fam = fams["V"][i % len(fams["V"])]
            one("V", fam, counters, i)
    present = [c for c in "NLB" if fams[c]]
    pattern = [c for c in INVALID_PATTERN if c in present] or present
    cnt = {c: 0 for c in "NLB"}
    for j in range(invalid):
        if not pattern:
            break
        c = pattern[j % len(pattern)]
        fam = fams[c][cnt[c] % len(fams[c])]
        cnt[c] += 1
        one(c, fam, counters, j)
    for c in "VNLB":
        stats[c]["structural_n"] = len(stats[c]["structural"])
    return records, stats, fam_stats


def corpus_sha(records, cls=None) -> str:
    h = hashlib.sha256()
    for r in records:
        if cls is not None and r["fclass"] != cls:
            continue
        h.update(fio.dumps(r).encode("utf-8"))
        h.update(b"\n")
    return h.hexdigest()


# ------------------------------------------------------------------ reductions (minimizer)

def _stmt_lists(node, out, path=()):
    """Collect (list, owner) pairs of statement bodies of fn/for/arm nodes of a repr."""
    if isinstance(node, dict):
        if node.get("_") in ("Fn", "For", "Arm") and isinstance(node.get("body"), list):
            out.append(node["body"])
        for v in node.values():
            _stmt_lists(v, out)
    elif isinstance(node, list):
        for v in node:
            _stmt_lists(v, out)


def _count_name(node, name) -> int:
    if isinstance(node, dict):
        return sum(_count_name(v, name) for v in node.values())
    if isinstance(node, list):
        return sum(_count_name(v, name) for v in node)
    return 1 if node == name else 0


def _exprs(node, out):
    if isinstance(node, dict):
        if node.get("_") in ("Bin", "IfE", "Arr", "Str"):
            out.append(node)
        for v in node.values():
            _exprs(v, out)
    elif isinstance(node, list):
        for v in node:
            _exprs(v, out)


def _copy(x):
    if isinstance(x, dict):
        return {k: _copy(v) for k, v in x.items()}
    if isinstance(x, list):
        return [_copy(v) for v in x]
    return x


def _prog_reductions(program: dict):
    """Candidate programs (reprs) smaller than ``program``; the model recomputes expectations."""
    cands = []
    lists = []
    _stmt_lists(program, lists)
    for li in range(len(lists)):
        for si in range(len(lists[li])):
            c = _copy(program)
            ls = []
            _stmt_lists(c, ls)
            del ls[li][si]
            cands.append(c)
    decls = program["decls"]
    for di, d in enumerate(decls):
        name = d.get("name") if isinstance(d, dict) else None
        if name and name != "Main" and _count_name(program, name) <= 2:
            c = _copy(program)
            del c["decls"][di]
            cands.append(c)
    exprs = []
    _exprs(program, exprs)
    for ei in range(len(exprs)):
        e = exprs[ei]
        kind = e["_"]
        options = []
        if kind == "Bin":
            options = [e["left"], e["right"]]
        elif kind == "IfE":
            options = [e["a"], e["b"]]
        elif kind == "Arr" and len(e["items"]) > 1:
            options = [{"_": "Arr", "items": e["items"][:-1]}]
        elif kind == "Str" and len(e["parts"]) > 1:
            options = [{"_": "Str", "parts": e["parts"][:-1]}]
        for oi in range(len(options)):
            c = _copy(program)
            es = []
            _exprs(c, es)
            target = es[ei]
            repl = _copy(options[oi])
            target.clear()
            target.update(repl)
            cands.append(c)
    return cands


def reductions(rep: dict) -> list:
    """Candidate smaller reps of ``rep`` (pure; the model recomputes their expectations)."""
    out = []
    if rep["cls"] == "L":
        lit = rep["lit"]
        for i in range(len(lit["pads"])):
            c = _copy(rep)
            del c["lit"]["pads"][i]
            out.append(c)
        pre = lit["head"]
        marker = lit["head"].index('"') + 1
        text = pre[marker:]
        if text:
            c = _copy(rep)
            c["lit"]["head"] = pre[:marker] + text[1:]
            c["lit"]["positive_text"] = lit["positive_text"][1:]
            out.append(c)
        return out
    for prog in _prog_reductions(rep["program"]):
        c = _copy(rep)
        c["program"] = prog
        out.append(c)
    return out


# ------------------------------------------------------------------ statistics helpers

def family_table(phase: str, complete) -> dict:
    closed = barrier.validate_manifest(phase, complete)
    return {c: [f.name for f in families_for(c, closed)] for c in "VNLB"}
