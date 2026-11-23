"""Independent model of F0-0004 (class MODELO).

A typed representation of programs (never Nexo text that is executed), a serializer
that returns the byte offset map of every node, a checker that derives diagnostics
from K's rules and an evaluator for run outcomes.  Strings are byte strings checked
by ``utf8ref``; integers are unbounded Python ints with an explicit I64 range.
No expected value is taken from a binary, from the compiler sources or from a prototype.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import diagnostics, literal, utf8ref

I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
UNIT = "()"

# ============================================================ text operations (K section 3)


def from_i64(n: int) -> bytes:
    """Canonical decimal ASCII, minimum I64 included."""
    if n == 0:
        return b"0"
    digits = []
    m = -n if n < 0 else n
    while m > 0:
        m, d = divmod(m, 10)
        digits.append(48 + d)
    if n < 0:
        digits.append(45)
    digits.reverse()
    return bytes(digits)


def parse_i64(s: bytes):
    """Grammar ``-?[0-9]+`` is validated completely before the range (K section 3)."""
    i = 1 if len(s) > 0 and s[0] == 45 else 0
    if i >= len(s):
        return ("Err", -9101)
    for k in range(i, len(s)):
        if s[k] < 48 or s[k] > 57:
            return ("Err", -9101)
    value = 0
    for k in range(i, len(s)):
        value = value * 10 + (s[k] - 48)
    if i == 1:
        value = -value
    if value < I64_MIN or value > I64_MAX:
        return ("Err", -9102)
    return ("Ok", value)


def slice_bytes(s: bytes, start: int, end: int):
    """Range first (0 <= start <= end <= len), then both UTF-8 boundaries."""
    if not (0 <= start and start <= end and end <= len(s)):
        return ("Err", -9110)
    if not (utf8ref.is_boundary(s, start) and utf8ref.is_boundary(s, end)):
        return ("Err", -9111)
    return ("Ok", s[start:end])


def scalars_of(s: bytes) -> list:
    return utf8ref.decode(s)


def string_cmp(a: bytes, b: bytes) -> int:
    return utf8ref.compare(a, b)


def compare_op(op: str, a: bytes, b: bytes) -> bool:
    """One lexicographic comparison; ``!=`` negates ``==`` (K section 3)."""
    c = string_cmp(a, b)
    if op == "==":
        return c == 0
    if op == "!=":
        return c != 0
    if op == "<":
        return c < 0
    if op == "<=":
        return c <= 0
    if op == ">":
        return c > 0
    if op == ">=":
        return c >= 0
    raise ValueError(op)


# ============================================================ types (semantic form)
# atoms are strings; composites are tuples ("Array", T), ("Option", T), ("Result", T, E)

UNKNOWN = "?"


def show_type(t, qualify=False) -> str:
    if isinstance(t, str):
        return t
    return t[0] + "<" + ",".join(show_type(x, qualify) for x in t[1:]) + ">"


def diag_type(t, nominals) -> str:
    """Type as printed in diagnostics: user types are ``package.T``."""
    if isinstance(t, str):
        return "package." + t if t in nominals else t
    return t[0] + "<" + ",".join(diag_type(x, nominals) for x in t[1:]) + ">"


@dataclass(eq=False)
class TRef:
    name: str
    args: list = field(default_factory=list)


def ty(text: str) -> TRef:
    """Parse the type notation ``Array<S>``, ``Result<S,I64>``, ``()``."""
    pos = [0]

    def parse() -> TRef:
        start = pos[0]
        if text.startswith("()", pos[0]):
            pos[0] += 2
            return TRef("()")
        while pos[0] < len(text) and (text[pos[0]].isalnum() or text[pos[0]] in "_."):
            pos[0] += 1
        node = TRef(text[start:pos[0]])
        if pos[0] < len(text) and text[pos[0]] == "<":
            pos[0] += 1
            node.args.append(parse())
            while text[pos[0]] == ",":
                pos[0] += 1
                node.args.append(parse())
            pos[0] += 1
        return node

    return parse()


def sem(t: TRef):
    if not t.args:
        return t.name
    return (t.name,) + tuple(sem(a) for a in t.args)


# ============================================================ AST


@dataclass(eq=False)
class Lit:
    value: object


@dataclass(eq=False)
class Str:
    parts: list


@dataclass(eq=False)
class Name:
    name: str


@dataclass(eq=False)
class Arg:
    expr: object
    label: str | None = None
    mode: str | None = None


@dataclass(eq=False)
class Call:
    callee: str
    args: list = field(default_factory=list)
    prefix: str | None = None


@dataclass(eq=False)
class MCall:
    recv: object
    name: str
    args: list = field(default_factory=list)
    prefix: str | None = None


@dataclass(eq=False)
class Field_:
    base: object
    name: str


@dataclass(eq=False)
class Idx:
    base: object
    index: object


@dataclass(eq=False)
class Rng:
    """Range selector ``lo..<hi`` (K 3: ``s[a..<b]`` is range/later_adr)."""
    lo: object
    hi: object


@dataclass(eq=False)
class Bin:
    op: str
    left: object
    right: object
    sp: bool = False


@dataclass(eq=False)
class Arr:
    items: list


@dataclass(eq=False)
class Var:
    ty_name: str
    variant: str
    args: list | None = None
    called: bool = False


@dataclass(eq=False)
class Mv:
    expr: object


@dataclass(eq=False)
class Try:
    expr: object


@dataclass(eq=False)
class IfE:
    cond: object
    a: object
    b: object


@dataclass(eq=False)
class Raw:
    text: str
    t: object = UNKNOWN


@dataclass(eq=False)
class Let:
    name: str
    init: object
    mut: bool = False
    ty: TRef | None = None


@dataclass(eq=False)
class Assign:
    target: object
    expr: object
    op: str = "="


@dataclass(eq=False)
class ExprS:
    expr: object


@dataclass(eq=False)
class For:
    var: str
    iter: object
    body: list
    edit: bool = False
    ml: bool = False


@dataclass(eq=False)
class Arm:
    pattern: object
    body: list


@dataclass(eq=False)
class MatchS:
    scrut: object
    arms: list
    move: bool = False


@dataclass(eq=False)
class Ret:
    expr: object


@dataclass(eq=False)
class PVar:
    ty_name: str
    variant: str
    subs: list | None = None


@dataclass(eq=False)
class PBind:
    name: str
    mode: str | None = None


@dataclass(eq=False)
class PWild:
    pass


@dataclass(eq=False)
class PNone:
    pass


@dataclass(eq=False)
class PSome:
    sub: object


@dataclass(eq=False)
class Fld:
    name: str
    t: TRef


@dataclass(eq=False)
class Vnt:
    name: str
    payload: list = field(default_factory=list)


@dataclass(eq=False)
class Param:
    name: str
    mode: str | None
    t: TRef | None


@dataclass(eq=False)
class Struct:
    name: str
    fields: list
    derives: list = field(default_factory=list)
    pub: bool = False


@dataclass(eq=False)
class Enum:
    name: str
    variants: list
    derives: list = field(default_factory=list)
    pub: bool = False


@dataclass(eq=False)
class Fn:
    name: str
    params: list
    ret: TRef | None
    body: list
    with_: str | None = None
    pub: bool = False
    derives: list = field(default_factory=list)
    ml: bool = False


@dataclass(eq=False)
class Extend:
    ty_name: str
    methods: list


@dataclass(eq=False)
class UseExt:
    path: str


@dataclass(eq=False)
class Use:
    path: str
    alias: str | None = None
    pub: bool = False


@dataclass(eq=False)
class RawDecl:
    text: str
    kind: str


@dataclass(eq=False)
class Program:
    decls: list
    sep: str = ""
    final_lf: bool = False
    domain: str = "package"
    breaks: tuple = ()


# ---- terse builders (public: used by selftest to describe each K row)

def I(v):
    return Lit(v)


def B(v):
    return Lit(bool(v))


def S(*parts):
    return Str(list(parts))


def N(name):
    return Name(name)


def _arg(a):
    return a if isinstance(a, Arg) else Arg(a)


def C(callee, *args, prefix=None):
    return Call(callee, [_arg(a) for a in args], prefix)


def lab(label, expr):
    return Arg(expr, label=label)


def edit(expr):
    return Arg(expr, mode="edit")


def move(expr):
    return Arg(expr, mode="move")


def M(recv, name, *args, prefix=None):
    return MCall(recv, name, [_arg(a) for a in args], prefix)


def F(base, name):
    return Field_(base, name)


def V(ty_name, variant, *args, null=False, called=False):
    return Var(ty_name, variant, None if null else [_arg(a) for a in args], called)


def BIN(left, op, right, sp=False):
    return Bin(op, left, right, sp)


def ARR(*items):
    return Arr(list(items))


def RNG(lo, hi):
    return Rng(lo, hi)


def IDX(base, index):
    return Idx(base, index)


def TRY(expr):
    return Try(expr)


def IFE(cond, a, b):
    return IfE(cond, a, b)


def LET(name, init, mut=False, t=None):
    return Let(name, init, mut, ty(t) if t else None)


def VAR(name, init, t=None):
    return Let(name, init, True, ty(t) if t else None)


def ASG(target, expr, op="="):
    return Assign(target, expr, op)


def ES(expr):
    return ExprS(expr)


def FOR(var, it, body, edit_=False, ml=False):
    return For(var, it, body, edit_, ml)


def ARM(pattern, body=()):
    return Arm(pattern, list(body))


def MATCH(scrut, *arms, move_=False):
    return MatchS(scrut, list(arms), move_)


def STRUCT(name, fields, derive=(), pub=False):
    return Struct(name, [Fld(n, ty(t)) for n, t in fields], list(derive), pub)


def ENUM(name, variants, derive=(), pub=False):
    return Enum(name, [Vnt(n, [ty(t) for t in pl]) for n, pl in variants], list(derive), pub)


def PRM(name, mode, t):
    return Param(name, mode, ty(t) if t else None)


def FN(name, params, body, ret=None, with_=None, pub=False, derive=(), ml=False):
    return Fn(name, [PRM(*p) for p in params], ty(ret) if ret else None, list(body), with_, pub,
              list(derive), ml)


def EXT(ty_name, *methods):
    return Extend(ty_name, list(methods))


def METH(name, self_mode, body=(), params=(), ret=None):
    return FN(name, [("self", self_mode, None)] + list(params), body, ret)


# ============================================================ serializer with byte offset map


class Ser:
    def __init__(self):
        self.parts: list = []
        self.pos = 0
        self.spans: dict = {}

    def w(self, text: str):
        self.parts.append(text)
        self.pos += len(utf8ref.text(text))

    def mark(self, node, slot, start):
        self.spans[(id(node), slot)] = (start, self.pos)

    def at(self, node, slot, start, end):
        self.spans[(id(node), slot)] = (start, end)

    def text(self) -> str:
        return "".join(self.parts)


class Rendered:
    def __init__(self, source: str, spans: dict):
        self.source = source
        self.spans = spans

    def get(self, node, slot="all"):
        return self.spans[(id(node), slot)]


ESC = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t", "{": "{{", "}": "}}"}


def render(prog: Program) -> Rendered:
    s = Ser()
    n = len(prog.decls)
    for i, d in enumerate(prog.decls):
        _decl(s, d)
        if i + 1 < n:
            s.w("\n" if i in prog.breaks else prog.sep)
    if prog.final_lf:
        s.w("\n")
    return Rendered(s.text(), s.spans)


def _tref(s: Ser, t: TRef):
    st = s.pos
    if t.name == "()":
        s.w("()")
    else:
        s.w(t.name)
        if t.args:
            s.w("<")
            for i, a in enumerate(t.args):
                if i:
                    s.w(",")
                _tref(s, a)
            s.w(">")
    s.mark(t, "all", st)


def _derives(s: Ser, node, names):
    if not names:
        return
    st = s.pos
    s.w("@derive(")
    for i, name in enumerate(names):
        if i:
            s.w(",")
        a = s.pos
        s.w(name)
        s.at(node, ("derive", i), a, s.pos)
    s.w(")")
    s.mark(node, "attr", st)


def _decl(s: Ser, d):
    st = s.pos
    if isinstance(d, Struct):
        _derives(s, d, d.derives)
        if d.pub:
            s.w("pub ")
        k = s.pos
        s.w("struct")
        s.mark(d, "kw", k)
        s.w(" ")
        a = s.pos
        s.w(d.name)
        s.mark(d, "name", a)
        s.w("{")
        for f in d.fields:
            fa = s.pos
            s.w(f.name)
            s.mark(f, "name", fa)
            s.w(":")
            _tref(s, f.t)
            s.w(";")
        s.w("}")
    elif isinstance(d, Enum):
        _derives(s, d, d.derives)
        if d.pub:
            s.w("pub ")
        k = s.pos
        s.w("enum")
        s.mark(d, "kw", k)
        s.w(" ")
        a = s.pos
        s.w(d.name)
        s.mark(d, "name", a)
        s.w("{")
        for i, v in enumerate(d.variants):
            if i:
                s.w(",")
            va = s.pos
            s.w(v.name)
            s.mark(v, "name", va)
            if v.payload:
                s.w("(")
                for j, t in enumerate(v.payload):
                    if j:
                        s.w(",")
                    _tref(s, t)
                s.w(")")
        s.w("}")
    elif isinstance(d, Fn):
        _fn(s, d)
    elif isinstance(d, Extend):
        s.w("extend ")
        a = s.pos
        s.w(d.ty_name)
        s.mark(d, "name", a)
        s.w("{")
        for m in d.methods:
            _fn(s, m)
        s.w("}")
    elif isinstance(d, UseExt):
        s.w("use extension ")
        a = s.pos
        s.w(d.path)
        s.mark(d, "path", a)
        s.w(";")
    elif isinstance(d, Use):
        if d.pub:
            s.w("pub ")
        s.w("use ")
        a = s.pos
        s.w(d.path)
        s.mark(d, "path", a)
        last = d.path.split(".")[-1]
        s.at(d, "last", s.pos - len(last), s.pos)
        if d.alias:
            s.w(" as ")
            b = s.pos
            s.w(d.alias)
            s.mark(d, "alias", b)
        s.w(";")
    elif isinstance(d, RawDecl):
        first = d.text.split(" ")[0].rstrip(";{")
        s.at(d, "kw", s.pos, s.pos + len(first))
        s.w(d.text)
    else:
        raise TypeError(d)
    s.mark(d, "all", st)


def _params(s: Ser, f: Fn):
    s.w("(")
    for i, p in enumerate(f.params):
        if i:
            s.w(",")
        pa = s.pos
        s.w(p.name)
        s.mark(p, "name", pa)
        if p.mode is not None or p.t is not None:
            s.w(":")
            if p.mode:
                s.w(p.mode)
                if p.t is not None:
                    s.w(" ")
            if p.t is not None:
                _tref(s, p.t)
        s.mark(p, "all", pa)
    s.w(")")
    s.at(f, "params_end", s.pos, s.pos)


def _fn(s: Ser, f: Fn, indent=0):
    st = s.pos
    _derives(s, f, f.derives)
    if f.pub:
        s.w("pub ")
    s.w("fn ")
    a = s.pos
    s.w(f.name)
    s.mark(f, "name", a)
    _params(s, f)
    if f.ret is not None:
        s.w("->")
        _tref(s, f.ret)
    if f.with_:
        s.w(" with ")
        b = s.pos
        s.w(f.with_)
        s.mark(f, "with", b)
    s.w("{")
    _block(s, f.body, f.ml, 4 if f.ml else 0)
    if f.ml:
        s.w("\n")
    s.w("}")
    s.mark(f, "all", st)


def _block(s: Ser, stmts, ml: bool, indent: int):
    for st in stmts:
        if ml:
            s.w("\n" + " " * indent)
        _stmt(s, st, indent)


def _stmt(s: Ser, st, indent=0):
    a = s.pos
    if isinstance(st, Let):
        s.w("var " if st.mut else "let ")
        b = s.pos
        s.w(st.name)
        s.mark(st, "name", b)
        if st.ty is not None:
            s.w(":")
            _tref(s, st.ty)
        s.w("=")
        _expr(s, st.init)
        s.w(";")
    elif isinstance(st, Assign):
        _expr(s, st.target)
        s.w(st.op)
        _expr(s, st.expr)
        s.w(";")
    elif isinstance(st, ExprS):
        _expr(s, st.expr)
        s.w(";")
    elif isinstance(st, For):
        s.w("for ")
        if st.edit:
            b = s.pos
            s.w("edit")
            s.mark(st, "edit_kw", b)
            s.w(" ")
        b = s.pos
        s.w(st.var)
        s.mark(st, "name", b)
        s.w(" in ")
        _expr(s, st.iter)
        s.w("{")
        _block(s, st.body, st.ml, indent + 4)
        if st.ml:
            s.w("\n" + " " * indent)
        s.w("}")
    elif isinstance(st, MatchS):
        b = s.pos
        s.w("match")
        s.mark(st, "kw", b)
        s.w(" ")
        if st.move:
            s.w("move ")
        _expr(s, st.scrut)
        s.w("{")
        for i, arm in enumerate(st.arms):
            if i:
                s.w(",")
            c = s.pos
            _pat(s, arm.pattern)
            s.w("=>{")
            _block(s, arm.body, False, 0)
            s.w("}")
            s.mark(arm, "all", c)
        s.w("}")
    elif isinstance(st, Ret):
        s.w("return ")
        _expr(s, st.expr)
        s.w(";")
    else:
        raise TypeError(st)
    s.mark(st, "all", a)


def _pat(s: Ser, p):
    a = s.pos
    if isinstance(p, PVar):
        s.w(p.ty_name + "." + p.variant)
        if p.subs is not None:
            s.w("(")
            for i, sub in enumerate(p.subs):
                if i:
                    s.w(",")
                _pat(s, sub)
            s.w(")")
    elif isinstance(p, PBind):
        if p.mode:
            s.w(p.mode + " ")
        s.w(p.name)
    elif isinstance(p, PWild):
        s.w("_")
    elif isinstance(p, PNone):
        s.w("None")
    elif isinstance(p, PSome):
        s.w("Some(")
        _pat(s, p.sub)
        s.w(")")
    else:
        raise TypeError(p)
    s.mark(p, "all", a)


def _args(s: Ser, args):
    for i, a in enumerate(args):
        if i:
            s.w(",")
        st = s.pos
        if a.label:
            b = s.pos
            s.w(a.label)
            s.mark(a, "label", b)
            s.w(":")
        if a.mode:
            s.w(a.mode + " ")
        _expr(s, a.expr)
        s.mark(a, "all", st)


def _expr(s: Ser, e):
    a = s.pos
    if isinstance(e, Lit):
        if e.value is True:
            s.w("true")
        elif e.value is False:
            s.w("false")
        else:
            s.w(str(e.value))
    elif isinstance(e, Str):
        s.w('"')
        for part in e.parts:
            if isinstance(part, str):
                s.w("".join(ESC.get(ch, ch) for ch in part))
            else:
                s.w("{")
                _expr(s, part)
                s.w("}")
        s.w('"')
    elif isinstance(e, Name):
        s.w(e.name)
    elif isinstance(e, Call):
        if e.prefix:
            k = s.pos
            s.w(e.prefix)
            s.mark(e, "prefix_kw", k)
            s.w(" ")
        c = s.pos
        s.w(e.callee)
        s.mark(e, "callee", c)
        o = s.pos
        s.w("(")
        s.at(e, "open", o, s.pos)
        _args(s, e.args)
        s.w(")")
    elif isinstance(e, MCall):
        if e.prefix:
            k = s.pos
            s.w(e.prefix)
            s.mark(e, "prefix_kw", k)
            s.w(" ")
        r = s.pos
        _expr(s, e.recv)
        s.mark(e, "recv", r)
        s.w(".")
        n = s.pos
        s.w(e.name)
        s.mark(e, "name", n)
        s.w("(")
        _args(s, e.args)
        s.w(")")
    elif isinstance(e, Field_):
        _expr(s, e.base)
        s.w(".")
        n = s.pos
        s.w(e.name)
        s.mark(e, "name", n)
    elif isinstance(e, Rng):
        _expr(s, e.lo)
        s.w("..<")
        _expr(s, e.hi)
    elif isinstance(e, Idx):
        _expr(s, e.base)
        s.w("[")
        _expr(s, e.index)
        s.w("]")
    elif isinstance(e, Bin):
        _expr(s, e.left)
        if e.sp:
            s.w(" ")
        o = s.pos
        s.w(e.op)
        s.mark(e, "op", o)
        if e.sp:
            s.w(" ")
        _expr(s, e.right)
    elif isinstance(e, Arr):
        s.w("[")
        for i, item in enumerate(e.items):
            if i:
                s.w(",")
            _expr(s, item)
        s.w("]")
    elif isinstance(e, Var):
        p = s.pos
        s.w(e.ty_name + "." + e.variant)
        s.mark(e, "path", p)
        v = s.pos
        s.at(e, "name", s.pos - len(e.variant), s.pos)
        if e.args is not None:
            s.w("(")
            _args(s, e.args)
            s.w(")")
        elif e.called:
            s.w("()")
    elif isinstance(e, Mv):
        k = s.pos
        s.w("move")
        s.mark(e, "kw", k)
        s.w(" ")
        _expr(s, e.expr)
    elif isinstance(e, Try):
        _expr(s, e.expr)
        s.w("?")
    elif isinstance(e, IfE):
        s.w("if ")
        _expr(s, e.cond)
        s.w(" then ")
        _expr(s, e.a)
        s.w(" else ")
        _expr(s, e.b)
    elif isinstance(e, Raw):
        s.w(e.text)
    else:
        raise TypeError(e)
    s.mark(e, "all", a)


# ============================================================ checker (K sections 2-5, 9, 10)

BUILTIN_ATOMS = ("I64", "Bool", "String", UNIT)
CAP_SUB = {"Format": "missing_format", "PartialEq": "missing_partial_eq"}
STRING_METHODS = {"ByteLen": "I64", "ScalarLen": "I64", "IsEmpty": "Bool",
                  "Concat": "String", "SliceBytes": ("Result", "String", "I64"),
                  "Scalars": ("Array", "I64")}
DEFERRED_TEXT = ("Graphemes", "View")


class Req:
    """A diagnostic to be resolved against the byte offset map after rendering."""

    def __init__(self, code, sub=None, primary=None, related=(), data=None, **kw):
        self.code = code
        self.sub = sub
        self.primary = primary
        self.related = list(related)
        self.data = data or {}
        self.kw = kw


class VInfo:
    def __init__(self, name, t, node, slot, kind, mode=None):
        self.name = name
        self.t = t
        self.node = node
        self.slot = slot
        self.kind = kind          # let | var | param
        self.mode = mode          # param permission: view | edit | move | None
        self.consumed = None


def is_place(e) -> bool:
    if isinstance(e, Name):
        return True
    if isinstance(e, (Field_, Idx)):
        return is_place(e.base)
    return False


def root_of(e):
    while isinstance(e, (Field_, Idx)):
        e = e.base
    return e if isinstance(e, Name) else None


class Checker:
    def __init__(self, prog: Program, rendered: Rendered, limits=None):
        self.prog = prog
        self.r = rendered
        self.limits = limits or {}
        self.reqs: list = []
        self.structs = {d.name: d for d in prog.decls if isinstance(d, Struct)}
        self.enums = {d.name: d for d in prog.decls if isinstance(d, Enum)}
        self.nominals = {**self.structs, **self.enums}
        self.funcs = {d.name: d for d in prog.decls if isinstance(d, Fn)}
        self.exts: dict = {}
        self.activated = {d.path.split(".")[-1] for d in prog.decls if isinstance(d, UseExt)}
        for d in prog.decls:
            if isinstance(d, Extend):
                for m in d.methods:
                    self.exts.setdefault(d.ty_name, []).append((m, d))
        self.env: dict = {}
        self.fn: Fn | None = None
        self.loans: list = []
        self.copy_memo: dict = {}

    # ---------------------------------------------------- helpers
    def emit(self, *a, **kw):
        self.reqs.append(Req(*a, **kw))

    def span(self, key):
        node, slot = key
        return self.r.get(node, slot)

    def dt(self, t) -> str:
        return diag_type(t, self.nominals)

    def comps(self, name):
        d = self.nominals[name]
        if isinstance(d, Struct):
            return [sem(f.t) for f in d.fields]
        return [sem(t) for v in d.variants for t in v.payload]

    def copyable(self, t, seen=()) -> bool:
        if isinstance(t, str):
            if t in BUILTIN_ATOMS or t == UNKNOWN:
                return True
            if t == "MockResource":
                return False
            if t in self.nominals:
                if t in seen:
                    return True
                return all(self.copyable(c, seen + (t,)) for c in self.comps(t))
            return True
        return all(self.copyable(x, seen) for x in t[1:])

    def has_resource(self, t, seen=()) -> bool:
        if isinstance(t, str):
            if t == "MockResource":
                return True
            if t in self.nominals and t not in seen:
                return any(self.has_resource(c, seen + (t,)) for c in self.comps(t))
            return False
        return any(self.has_resource(x, seen) for x in t[1:])

    def lacking(self, t, cap, seen=()):
        """First component without capacity ``cap``: ('nominal', decl) | ('resource', None) | None."""
        if isinstance(t, str):
            if t in BUILTIN_ATOMS or t == UNKNOWN:
                return None
            if t == "MockResource":
                return ("resource", None)
            if t in self.nominals:
                d = self.nominals[t]
                if cap not in d.derives:
                    return ("nominal", d)
                return None
            return None
        for x in t[1:]:
            found = self.lacking(x, cap, seen)
            if found:
                return found
        return None

    def same(self, a, b) -> bool:
        if a == UNKNOWN or b == UNKNOWN:
            return True
        if isinstance(a, str) or isinstance(b, str):
            return a == b
        return a[0] == b[0] and len(a) == len(b) and all(self.same(x, y) for x, y in zip(a[1:], b[1:]))

    def need_cap(self, t, cap, primary, extra_related=()):
        found = self.lacking(t, cap)
        if found is None:
            return
        kind, decl = found
        if kind == "resource":
            # K 5.2: uses already rejected by A2/F0-0003 on resources keep NX-E0102/type_mismatch
            self.emit("NX-E0102", "type_mismatch", primary, [("found_type", primary)], {})
            return
        related = [("declaration", (decl, "name"))] + list(extra_related)
        self.emit("NX-E1403", CAP_SUB[cap], primary, related, {"required_capability": cap})

    # ---------------------------------------------------- entry point
    def run(self):
        if self.syntax_checks():
            return self.reqs
        self.decl_checks()
        for d in self.prog.decls:
            if isinstance(d, Fn):
                self.check_fn(d)
            elif isinstance(d, Extend):
                for m in d.methods:
                    self.check_fn(m, self_type=d.ty_name)
        return self.reqs

    # ---------------------------------------------------- parse-level exclusions
    def syntax_checks(self) -> bool:
        before = len(self.reqs)
        later = {"pending_scope": "later_adr"}
        for d in self.prog.decls:
            if isinstance(d, RawDecl):
                feature = {"mod": "modules_visibility", "trait": "traits_generics"}[d.kind]
                self.emit("NX-E0006", None, (d, "kw"), (), later, feature_id=feature,
                          stage="parse", operation="syntax.parse")
            if isinstance(d, (Struct, Enum, Fn)):
                for i, name in enumerate(d.derives):
                    if name not in ("Format", "PartialEq"):
                        self.emit("NX-E0006", None, (d, ("derive", i)), (), later,
                                  feature_id="attributes", stage="parse", operation="syntax.parse")
            if isinstance(d, Enum) and not d.variants:
                self.emit("NX-E0006", None, (d, "kw"), (), later, feature_id="user_types",
                          stage="parse", operation="syntax.parse")
            if isinstance(d, Fn):
                self.syntax_block(d.body)
            if isinstance(d, Extend):
                for m in d.methods:
                    self.syntax_block(m.body)
        return len(self.reqs) > before

    def syntax_block(self, stmts):
        later = {"pending_scope": "later_adr"}
        for st in stmts:
            for e in self.exprs_of(st):
                for sub in self.walk(e):
                    if isinstance(sub, Mv) and isinstance(sub.expr, Field_):
                        self.emit("NX-E0006", None, (sub.expr, "all"), [("context", (sub, "kw"))],
                                  later, feature_id="partial_transfer", stage="parse",
                                  operation="syntax.parse")
                    if isinstance(sub, MCall) and sub.prefix and not is_place(sub.recv):
                        ctx = "inout_place" if sub.prefix == "edit" else "consume_place"
                        tok = (sub.recv, "open") if isinstance(sub.recv, Call) else (sub.recv, "all")
                        self.emit("NX-E0002", None, tok, (), {}, parser_context=ctx,
                                  expected_tokens=["Place"], expected_tokens_kind="admissible_set",
                                  token_kind="symbol", token_text="(", prefix_from=(sub, "prefix_kw"),
                                  stage="parse", operation="syntax.parse")
            if isinstance(st, For):
                if st.edit:
                    self.emit("NX-E0006", None, (st, "edit_kw"), (), later,
                              feature_id="for_iteration", stage="parse", operation="syntax.parse")
                self.syntax_block(st.body)
            if isinstance(st, MatchS):
                for arm in st.arms:
                    self.syntax_block(arm.body)

    def exprs_of(self, st):
        if isinstance(st, Let):
            return [st.init]
        if isinstance(st, Assign):
            return [st.target, st.expr]
        if isinstance(st, ExprS):
            return [st.expr]
        if isinstance(st, For):
            return [st.iter]
        if isinstance(st, MatchS):
            return [st.scrut]
        if isinstance(st, Ret):
            return [st.expr]
        return []

    def walk(self, e):
        yield e
        kids = []
        if isinstance(e, Call):
            kids = [a.expr for a in e.args]
        elif isinstance(e, MCall):
            kids = [e.recv] + [a.expr for a in e.args]
        elif isinstance(e, Field_):
            kids = [e.base]
        elif isinstance(e, Idx):
            kids = [e.base, e.index]
        elif isinstance(e, Rng):
            kids = [e.lo, e.hi]
        elif isinstance(e, Bin):
            kids = [e.left, e.right]
        elif isinstance(e, Arr):
            kids = e.items
        elif isinstance(e, Var) and e.args:
            kids = [a.expr for a in e.args]
        elif isinstance(e, (Mv, Try)):
            kids = [e.expr]
        elif isinstance(e, IfE):
            kids = [e.cond, e.a, e.b]
        elif isinstance(e, Str):
            kids = [p for p in e.parts if not isinstance(p, str)]
        for k in kids:
            yield from self.walk(k)

    # ---------------------------------------------------- declarations
    def decl_checks(self):
        count = 0
        limit = self.limits.get("nominal_members")
        for d in self.prog.decls:
            if isinstance(d, (Struct, Enum)):
                names = {}
                members = d.fields if isinstance(d, Struct) else d.variants
                for m in members:
                    if m.name in names:
                        self.emit("NX-E0104", None, (m, "name"), [("previous_declaration", (names[m.name], "name"))],
                                  {"declaration_kind": "field" if isinstance(d, Struct) else "variant"})
                    else:
                        names[m.name] = m
                if limit is not None:
                    items = []
                    if isinstance(d, Struct):
                        items = [((f, "name")) for f in d.fields]
                    else:
                        for v in d.variants:
                            items.append((v, "name"))
                            items.extend((t, "all") for t in v.payload)
                    for key in items:
                        count += 1
                        if count > limit:
                            self.emit("NX-E0003", None, key, (), {
                                "budget_kind": "nominal_members", "budget_scope": "compilation",
                                "incomplete_analysis": "types", "limit": limit, "observed": count},
                                stage="types", operation="analysis.limit",
                                profile_id=self.limits.get("profile_id"))
                            break
                    if count > (limit or 0):
                        break
        self.derive_checks()
        self.cycle_checks()
        self.visibility_checks()
        for tname, entries in self.exts.items():
            seen = {}
            for m, _ in entries:
                if m.name in seen:
                    self.emit("NX-E1322", None, (m, "name"), [("previous_declaration", (seen[m.name], "name"))])
                else:
                    seen[m.name] = m

    def derive_checks(self):
        for d in self.prog.decls:
            if not isinstance(d, (Struct, Enum, Fn)) or not d.derives:
                continue
            known = [(i, n) for i, n in enumerate(d.derives) if n in CAP_SUB]
            first = {}
            for i, n in known:
                if n in first:
                    self.emit("NX-E1404", "duplicate_derive", (d, ("derive", i)),
                              [("previous_declaration", (d, ("derive", first[n])))])
                else:
                    first[n] = i
            if isinstance(d, Fn):
                for i, n in known:
                    self.emit("NX-E1404", "invalid_derive_target", (d, ("derive", i)),
                              [("context", (d, "name"))])
                continue
            for n, i in first.items():
                for t in self.member_trefs(d):
                    found = self.lacking(sem(t), n)
                    if found:
                        kind, decl = found
                        related = [("contract", (d, ("derive", i)))]
                        if kind == "nominal":
                            related.append(("declaration", (decl, "name")))
                        self.emit("NX-E1403", CAP_SUB[n], (t, "all"), related, {"required_capability": n})
                        break

    def member_trefs(self, d):
        if isinstance(d, Struct):
            return [f.t for f in d.fields]
        return [t for v in d.variants for t in v.payload]

    def tref_nominals(self, t: TRef, inline_only=True):
        """Nominal references of a type reference in order; Array breaks the layout."""
        out = []
        if t.name in self.nominals:
            out.append((t.name, t, True))
        for a in t.args:
            if t.name == "Array":
                out.extend((n, r, False) for n, r, _ in self.tref_nominals(a))
            else:
                out.extend(self.tref_nominals(a))
        return out

    def cycle_checks(self):
        decls = [d for d in self.prog.decls if isinstance(d, (Struct, Enum))]
        edges = {d.name: [] for d in decls}
        array_edges = {d.name: [] for d in decls}
        for d in decls:
            for t in self.member_trefs(d):
                for name, ref, inline in self.tref_nominals(t):
                    (edges if inline else array_edges)[d.name].append((name, ref))
        index = {}
        low = {}
        stack = []
        on = set()
        sccs = []
        counter = [0]

        def strong(v):
            index[v] = low[v] = counter[0]
            counter[0] += 1
            stack.append(v)
            on.add(v)
            for w, _ in edges[v]:
                if w not in index:
                    strong(w)
                    low[v] = min(low[v], low[w])
                elif w in on:
                    low[v] = min(low[v], index[w])
            if low[v] == index[v]:
                comp = []
                while True:
                    w = stack.pop()
                    on.discard(w)
                    comp.append(w)
                    if w == v:
                        break
                sccs.append(comp)

        for d in decls:
            if d.name not in index:
                strong(d.name)
        found = []
        for comp in sccs:
            members = set(comp)
            cyclic = len(comp) > 1 or any(w == comp[0] for w, _ in edges[comp[0]])
            if not cyclic:
                continue
            best = None
            for start in comp:
                for cyc in self.simple_cycles(start, members, edges):
                    norm = self.rotate(cyc)
                    key = (len(norm), tuple(self.span((ref, "all"))[0] for _, _, ref in norm))
                    if best is None or key < best[0]:
                        best = (key, norm)
            norm = best[1]
            start_decl = self.nominals[norm[0][0]]
            found.append((self.span((norm[-1][2], "all"))[0], norm, start_decl))
        for _, norm, start_decl in sorted(found, key=lambda x: x[0]):
            self.emit("NX-E1323", "recursive_value_layout", (norm[-1][2], "all"),
                      [("declaration", (start_decl, "name"))])
        # a cycle that only exists through Array is a deferred nominal (K 4.2)
        in_cycle = {n for _, norm, _ in found for n, _, _ in norm}
        for d in decls:
            if d.name in in_cycle:
                continue
            for name, ref in array_edges[d.name]:
                if self.reaches(name, d.name, edges, array_edges):
                    self.emit("NX-E0006", None, (ref, "all"), (), {"pending_scope": "later_adr"},
                              feature_id="user_types")
                    break

    def reaches(self, a, b, edges, array_edges, seen=None):
        seen = seen or set()
        if a == b:
            return True
        if a in seen or a not in edges:
            return False
        seen.add(a)
        return any(self.reaches(w, b, edges, array_edges, seen)
                   for w, _ in edges[a] + array_edges[a])

    def simple_cycles(self, start, members, edges):
        out = []

        def dfs(node, path, visited):
            for w, ref in edges[node]:
                if w not in members:
                    continue
                if w == start:
                    out.append(path + [(node, w, ref)])
                elif w not in visited:
                    dfs(w, path + [(node, w, ref)], visited | {w})

        dfs(start, [], {start})
        return out

    def rotate(self, cyc):
        offsets = [self.span((self.nominals[a], "name"))[0] for a, _, _ in cyc]
        i = offsets.index(min(offsets))
        return cyc[i:] + cyc[:i]

    def visibility_checks(self):
        for d in self.prog.decls:
            if isinstance(d, Fn) and d.pub:
                if d.ret is None:
                    self.emit("NX-E1310", "missing_signature_type", (d, "params_end"),
                              [("declaration", (d, "name"))])
                if d.with_ is None:
                    self.emit("NX-E1310", "missing_effect_bound", (d, "name"))
                trefs = [p.t for p in d.params if p.t is not None] + ([d.ret] if d.ret else [])
                for t in trefs:
                    self.private_ref(t)
            if isinstance(d, Enum) and d.pub:
                for v in d.variants:
                    for t in v.payload:
                        self.private_ref(t)
            if isinstance(d, Use):
                last = d.path.split(".")[-1]
                decl = self.nominals.get(last)
                if d.pub and decl is not None and not decl.pub:
                    self.emit("NX-E1308", "private_reexport", (d, "last"),
                              [("declaration", (decl, "name"))])
                if d.alias and d.alias in self.nominals and d.alias != last:
                    self.emit("NX-E1309", "alias_collision", (d, "alias"),
                              [("previous_declaration", (self.nominals[d.alias], "name"))])

    def private_ref(self, t: TRef):
        for name, ref, _ in self.tref_nominals(t):
            decl = self.nominals[name]
            if not decl.pub:
                self.emit("NX-E1310", "unexported_type", (ref, "all"), [("declaration", (decl, "name"))])
                return

    # ---------------------------------------------------- bodies
    def check_fn(self, f: Fn, self_type=None):
        self.fn = f
        self.env = {}
        self.loans = []
        for p in f.params:
            t = sem(p.t) if p.t is not None else (self_type or UNKNOWN)
            self.env[p.name] = VInfo(p.name, t, p, "name", "param", p.mode)
        self.block(f.body)

    def block(self, stmts):
        saved = dict(self.env)
        for st in stmts:
            self.stmt(st)
        self.env = saved

    def stmt(self, st):
        if isinstance(st, Let):
            self.let(st)
        elif isinstance(st, Assign):
            self.assign(st)
        elif isinstance(st, ExprS):
            self.ty(st.expr)
        elif isinstance(st, Ret):
            self.ty(st.expr)
        elif isinstance(st, For):
            self.for_(st)
        elif isinstance(st, MatchS):
            self.match(st)

    def let(self, st: Let):
        it = self.ty(st.init)
        t = it
        if st.ty is not None:
            annotated = sem(st.ty)
            self.array_resource(st.ty)
            if not self.same(annotated, it):
                self.emit("NX-E0102", "type_mismatch", (st.init, "all"),
                          [("expected_type", (st.ty, "all")), ("found_type", (st.init, "all"))],
                          {"expected_type": self.dt(annotated), "found_type": self.dt(it)})
            t = annotated
        self.own(st.init)
        self.env[st.name] = VInfo(st.name, t, st, "name", "var" if st.mut else "let")

    def array_resource(self, t: TRef):
        if t.name == "Array" and t.args and self.has_resource(sem(t.args[0])):
            self.emit("NX-E0006", None, (t, "all"), (), {"pending_scope": "later_adr"},
                      feature_id="resource_array")
            return
        for a in t.args:
            self.array_resource(a)

    def own(self, e):
        """Owning context: copy a copyable place, require ``move`` for a non-copyable one."""
        if isinstance(e, Mv):
            inner = e.expr
            if isinstance(inner, Name) and inner.name in self.env:
                self.env[inner.name].consumed = (e, "all")
            return
        if isinstance(e, Name) and e.name in self.env:
            info = self.env[e.name]
            if info.consumed:
                return
            if not self.copyable(info.t):
                self.emit("NX-E0201", None, (e, "all"), [("declaration", (info.node, info.slot))])
            elif self.fn is not None and self.fn.with_ == "no_alloc" and self.has_string(info.t):
                self.effect_copy(e, info)

    def has_string(self, t) -> bool:
        if isinstance(t, str):
            return t == "String" or (t in self.nominals and any(self.has_string(c) for c in self.comps(t)))
        return any(self.has_string(x) for x in t[1:])

    def effect_copy(self, e, info):
        origin = self.span((self.fn, "with"))
        ident = ("parameter:" if info.kind == "param" else "binding:") + str(self.span((info.node, info.slot))[0])
        self.emit("NX-E0412", None, (e, "all"), [("contract", (self.fn, "with"))],
                  {"effect_model": "f0-0004-sim-v1", "effect_evidence": "known_effect",
                   "contract_origin": {"file": "@file", "start": origin[0], "end": origin[1]},
                   "required_effect": "alloc"},
                  stage="effects", operation="value.copy",
                  chain=[("copy", "value.copy", (e, "all"), {"kind": ident.split(":")[0], "id": ident})])

    def root_info(self, target):
        root = root_of(target)
        if root is None:
            return None, None
        return root, self.env.get(root.name)

    def assign(self, st: Assign):
        tt = self.ty(st.target)
        et = self.ty(st.expr)
        if st.op != "=":
            # K 3: the five compound operators stay I64; the first incompatible operand (A2 13.17)
            for side, t in ((st.target, tt), (st.expr, et)):
                if t not in ("I64", UNKNOWN):
                    self.emit("NX-E0102", "type_mismatch", (side, "all"), [("found_type", (side, "all"))],
                              {"expected_type": "I64", "found_type": self.dt(t)})
                    break
        self.own(st.expr)
        root, info = self.root_info(st.target)
        if info is None:
            return
        if True:
            if info.kind == "let":
                self.emit("NX-E0105", None, (root, "all"), [("declaration", (info.node, info.slot))])
                return
            if info.kind == "param" and info.mode == "view":
                ident = "parameter:" + str(self.span((info.node, info.slot))[0])
                ent = {"kind": "parameter", "id": ident}
                self.emit("NX-E1105", "permission_escalation", (root, "all"),
                          [("declaration", (info.node, info.slot))],
                          {"origin": ent, "parameter": ent, "active_permission": "read",
                           "requested_access": "write"},
                          chain=[("borrow", "borrow.read", (info.node, info.slot), ent)])
                return
            for loan_root, origin_key, bind in self.loans:
                if loan_root == root.name:
                    start = self.span(origin_key)[0]
                    bid = {"kind": "binding", "id": "binding:" + str(self.span((bind.node, bind.slot))[0])}
                    loan = {"kind": "loan", "id": f"loan:{start}:0"}
                    self.emit("NX-E1105", "active_borrow_conflict", (root, "all"),
                              [("declaration", (bind.node, bind.slot)), ("origin", origin_key)],
                              {"origin": bid, "binding": bid, "loan": loan,
                               "active_permission": "read", "requested_access": "write"},
                              chain=[("borrow", "borrow.read", origin_key, loan)])
                    return

    def for_(self, st: For):
        it = self.ty(st.iter)
        elem = UNKNOWN
        if isinstance(it, tuple) and it[0] == "Array":
            elem = it[1]
        elif it != UNKNOWN:
            self.emit("NX-E0102", "type_mismatch", (st.iter, "all"), [("found_type", (st.iter, "all"))],
                      {"expected_type": "domain:iterable", "found_type": self.dt(it)})
        pushed = False
        if isinstance(st.iter, Name) and st.iter.name in self.env and isinstance(it, tuple):
            self.loans.append((st.iter.name, (st.iter, "all"), self.env[st.iter.name]))
            pushed = True
        saved = dict(self.env)
        self.env[st.var] = VInfo(st.var, elem, st, "name", "let")
        self.block(st.body)
        self.env = saved
        if pushed:
            self.loans.pop()

    def match(self, st: MatchS):
        t = self.ty(st.scrut)
        covered = set()
        total = False
        enum = self.enums.get(t) if isinstance(t, str) else None
        for arm in st.arms:
            p = arm.pattern
            saved = dict(self.env)
            if isinstance(p, PVar) and enum is not None:
                vnt = next((v for v in enum.variants if v.name == p.variant), None)
                if vnt is not None:
                    found = len(p.subs) if p.subs is not None else 0
                    if found != len(vnt.payload):
                        total = True
                        self.emit("NX-E1325", "constructor_arity", (p, "all"), (),
                                  {"constructor": f"package.{enum.name}.{vnt.name}",
                                   "expected_arity": len(vnt.payload), "found_arity": found})
                    else:
                        if all(isinstance(s, (PBind, PWild)) for s in (p.subs or [])):
                            covered.add(vnt.name)
                        for sub, pt in zip(p.subs or [], vnt.payload):
                            if isinstance(sub, PBind):
                                self.env[sub.name] = VInfo(sub.name, sem(pt), sub, "all", "let")
            elif isinstance(p, (PWild, PBind)):
                total = True
            elif isinstance(p, PSome) and isinstance(p.sub, PBind) and isinstance(t, tuple):
                self.env[p.sub.name] = VInfo(p.sub.name, t[1], p.sub, "all", "let")
            self.block(arm.body)
            self.env = saved
        if enum is not None and not total:
            for v in enum.variants:
                if v.name not in covered:
                    wit = f"package.{enum.name}.{v.name}"
                    if v.payload:
                        wit += "(" + ",".join(self.witness(sem(x)) for x in v.payload) + ")"
                    self.emit("NX-E1304", "non_exhaustive", (st, "kw"), (), {"missing_pattern": wit})
                    break

    def witness(self, t) -> str:
        if t == "I64":
            return "0"
        if t == "Bool":
            return "false"
        return "_"

    # ---------------------------------------------------- expressions
    def ty(self, e):
        if isinstance(e, Lit):
            return "Bool" if isinstance(e.value, bool) else "I64"
        if isinstance(e, Name):
            info = self.env.get(e.name)
            if info is None:
                return UNKNOWN
            if info.consumed is not None:
                self.consumed_use(e, info)
            return info.t
        if isinstance(e, Str):
            return self.str_(e)
        if isinstance(e, Mv):
            return self.ty(e.expr)
        if isinstance(e, Try):
            t = self.ty(e.expr)
            return t[1] if isinstance(t, tuple) and t[0] == "Result" else UNKNOWN
        if isinstance(e, IfE):
            self.ty(e.cond)
            self.ty(e.b)
            return self.ty(e.a)
        if isinstance(e, Raw):
            return e.t
        if isinstance(e, Arr):
            items = [self.ty(i) for i in e.items]
            return ("Array", items[0] if items else UNKNOWN)
        if isinstance(e, Field_):
            return self.field(e)
        if isinstance(e, Idx):
            bt = self.ty(e.base)
            if isinstance(e.index, Rng):
                # K 3: the slice precedes the rejection of the child range and of the indexing
                self.emit("NX-E0006", None, (e, "all"), (), {"pending_scope": "later_adr"}, feature_id="range")
                return UNKNOWN
            self.ty(e.index)
            if bt == "String":
                self.emit("NX-E0102", "not_indexable", (e.base, "all"), [("found_type", (e.base, "all"))],
                          {"expected_type": "domain:array", "found_type": "String"})
                return UNKNOWN
            return bt[1] if isinstance(bt, tuple) and bt[0] == "Array" else UNKNOWN
        if isinstance(e, Bin):
            return self.bin(e)
        if isinstance(e, Var):
            return self.variant(e)
        if isinstance(e, MCall):
            return self.mcall(e)
        if isinstance(e, Call):
            return self.call(e)
        return UNKNOWN

    def consumed_use(self, e, info):
        ident = "binding:" + str(self.span((info.node, info.slot))[0])
        key = info.consumed
        self.emit("NX-E0202", "definitely_consumed", (e, "all"),
                  [("declaration", (info.node, info.slot)), ("consumption", key)], {},
                  chain=[("drop", "value.drop", key, {"kind": "binding", "id": ident})])
        info.consumed = None

    def str_(self, e: Str):
        if self.fn is not None and self.fn.with_ == "no_alloc":
            self.emit("NX-E0412", None, (e, "all"), [("contract", (self.fn, "with"))],
                      {"effect_model": "f0-0004-sim-v1", "effect_evidence": "known_effect",
                       "contract_origin": {"file": "@file", "start": self.span((self.fn, "with"))[0],
                                           "end": self.span((self.fn, "with"))[1]},
                       "required_effect": "alloc"},
                      stage="effects", operation="value.build_string",
                      chain=[("effect", "value.build_string", (e, "all"), {"kind": "type", "id": "String"})])
        for part in e.parts:
            if not isinstance(part, str):
                t = self.ty(part)
                self.need_cap(t, "Format", (part, "all"))
        return "String"

    def field(self, e: Field_):
        bt = self.ty(e.base)
        if isinstance(bt, str) and bt in self.structs:
            d = self.structs[bt]
            f = next((x for x in d.fields if x.name == e.name), None)
            if f is None:
                self.emit("NX-E1402", "unknown_member", (e, "name"), [("declaration", (d, "name"))],
                          {"member_name": e.name, "receiver_type": self.dt(bt)})
                return UNKNOWN
            return sem(f.t)
        return UNKNOWN

    def variant(self, e: Var):
        d = self.enums.get(e.ty_name)
        if d is None:
            return UNKNOWN
        v = next((x for x in d.variants if x.name == e.variant), None)
        if v is None:
            self.emit("NX-E1402", "unknown_member", (e, "name"), [("declaration", (d, "name"))],
                      {"member_name": e.variant, "receiver_type": self.dt(e.ty_name)})
            return UNKNOWN
        if e.called and e.args is None:
            self.emit("NX-E0111", "non_callable_target", (e, "path"), (), {},
                      stage="types", operation="call.bind")
            return e.ty_name
        for a in e.args or []:
            self.ty(a.expr)
            self.own(a.expr)
        return e.ty_name

    def bin(self, e: Bin):
        lt = self.ty(e.left)
        rt = self.ty(e.right)
        op = e.op
        if op in ("+", "-", "*", "/", "%"):
            for side, t in ((e.left, lt), (e.right, rt)):
                if t not in ("I64", UNKNOWN):
                    self.emit("NX-E0102", "type_mismatch", (side, "all"), [("found_type", (side, "all"))],
                              {"expected_type": "I64", "found_type": self.dt(t)})
                    break
            return "I64"
        if not self.same(lt, rt):
            self.emit("NX-E0102", "type_mismatch", (e.right, "all"),
                      [("expected_type", (e.left, "all")), ("found_type", (e.right, "all"))],
                      {"expected_type": self.dt(lt), "found_type": self.dt(rt)})
            return "Bool"
        if op in ("==", "!="):
            self.need_cap(lt, "PartialEq", (e, "op"))
        elif lt not in ("I64", "String", UNKNOWN):
            self.emit("NX-E0102", "type_mismatch", (e, "op"), [("found_type", (e.left, "all"))],
                      {"expected_type": "domain:ordered_scalar", "found_type": self.dt(lt)})
        return "Bool"

    def mcall(self, e: MCall):
        rt = self.ty(e.recv)
        for a in e.args:
            self.ty(a.expr)
        if rt == "String":
            if e.name in STRING_METHODS:
                return STRING_METHODS[e.name]
            if e.name in DEFERRED_TEXT:
                self.emit("NX-E0006", None, (e, "name"), (), {"pending_scope": "later_adr"},
                          feature_id="text")
                return UNKNOWN
            self.emit("NX-E1402", "unknown_member", (e, "name"), (),
                      {"member_name": e.name, "receiver_type": "String"})
            return UNKNOWN
        if isinstance(rt, str) and rt in self.nominals:
            entry = next(((m, b) for m, b in self.exts.get(rt, []) if m.name == e.name), None)
            if entry is None:
                self.emit("NX-E1402", "unknown_member", (e, "name"), [("declaration", (self.nominals[rt], "name"))],
                          {"member_name": e.name, "receiver_type": self.dt(rt)})
                return UNKNOWN
            m, _block = entry
            if rt not in self.activated:
                self.emit("NX-E1315", None, (e, "name"), [("declaration", (m, "name"))])
                return UNKNOWN
            self.self_mode(e, m)
            return sem(m.ret) if m.ret is not None else UNIT
        return UNKNOWN

    def self_mode(self, e: MCall, m: Fn):
        rank = {"read": 0, "inout": 1, "consume": 2}
        required = {"view": "read", "edit": "inout", "move": "consume"}[m.params[0].mode]
        requested = {None: "read", "edit": "inout", "move": "consume"}[e.prefix]
        if rank[requested] >= rank[required]:
            return
        data = {"required_mode": required, "requested_mode": requested}
        if is_place(e.recv):
            root = root_of(e.recv)
            info = self.env.get(root.name)
            related = [("declaration", (info.node, info.slot))] if info else []
            self.emit("NX-E1107", None, (e.recv, "all"), related, data)
        else:
            self.emit("NX-E1107", None, (e.recv, "all"), (), data)

    def call(self, e: Call):
        name = e.callee
        arg_types = []
        for a in e.args:
            arg_types.append(self.ty(a.expr))
        if name in self.structs:
            return self.constructor(e, self.structs[name])
        if name == "Print":
            if arg_types:
                self.need_cap(arg_types[0], "Format", (e.args[0].expr, "all"))
            return UNIT
        if name == "AssertEq":
            if len(arg_types) == 2 and self.same(arg_types[0], arg_types[1]):
                self.need_cap(arg_types[0], "PartialEq", (e.args[0].expr, "all"))
            return UNIT
        if name == "Drop":
            a = e.args[0].expr
            if isinstance(a, Field_):
                self.emit("NX-E0006", None, (a, "all"), [("context", (e, "callee"))],
                          {"pending_scope": "later_adr"}, feature_id="partial_transfer",
                          stage="arguments", operation="call.bind")
            elif isinstance(a, Name) and a.name in self.env:
                self.env[a.name].consumed = (e, "all")
            return UNIT
        if name == "Push":
            return UNIT
        if name == "Pop":
            t = arg_types[0]
            return ("Option", t[1]) if isinstance(t, tuple) else ("Option", UNKNOWN)
        if name == "Len":
            return "I64"
        if name == "JoinArrays":
            return arg_types[0]
        if name == "Some":
            return ("Option", arg_types[0] if arg_types else UNKNOWN)
        if name == "Ok":
            return ("Result", arg_types[0] if arg_types else UNKNOWN, UNKNOWN)
        if name == "Err":
            return ("Result", UNKNOWN, arg_types[0] if arg_types else UNKNOWN)
        if name in ("std.parse.I64", "parse.I64"):
            return ("Result", "I64", "I64")
        if name in ("std.text.FromI64", "text.FromI64"):
            return "String"
        if name == "MockOpen":
            return "MockResource"
        f = self.funcs.get(name)
        if f is not None:
            self.edit_overlap(e)
            return sem(f.ret) if f.ret is not None else UNIT
        return UNKNOWN

    def edit_overlap(self, e: Call):
        seen = {}
        for a in e.args:
            if a.mode != "edit":
                continue
            root = root_of(a.expr)
            if root is None or root.name not in self.env:
                continue
            if root.name in seen:
                first = seen[root.name]
                info = self.env[root.name]
                start = self.span((first, "all"))[0]
                bid = {"kind": "binding", "id": "binding:" + str(self.span((info.node, info.slot))[0])}
                loan = {"kind": "loan", "id": f"loan:{start}:0"}
                self.emit("NX-E1105", "overlapping_mutable_borrows", (a.expr, "all"),
                          [("declaration", (info.node, info.slot)), ("origin", (first, "all"))],
                          {"origin": bid, "binding": bid, "loan": loan,
                           "active_permission": "inout", "requested_access": "borrow_inout"},
                          chain=[("borrow", "borrow.inout", (first, "all"), loan)])
                return
            seen[root.name] = a

    def constructor(self, e: Call, d: Struct):
        fields = d.fields
        assigned = {}
        for i, a in enumerate([a for a in e.args if a.label is None]):
            if i < len(fields):
                assigned[fields[i].name] = a
        unknown = [a for a in e.args if a.label is not None and a.label not in [f.name for f in fields]]
        for a in e.args:
            if a.label is not None and a.label in [f.name for f in fields]:
                assigned[a.label] = a
        if unknown:
            for a in unknown:
                self.emit("NX-E0111", "unknown_argument_label", (a, "label"))
        else:
            missing = [f for f in fields if f.name not in assigned]
            if missing:
                self.emit("NX-E0111", "missing_argument", (e, "callee"), [("declaration", (missing[0], "name"))])
        for a in e.args:
            self.own(a.expr)
        return d.name


def check_program(prog: Program, file: str, limits=None):
    """Render ``prog`` and return (source, [diagnostic dicts]) from the model's rules."""
    rendered = render(prog)
    checker = Checker(prog, rendered, limits)
    reqs = checker.run()
    out = []
    for q in reqs:
        out.append(_resolve(q, rendered, file, checker, limits))
    return rendered.source, out


def _resolve(q: Req, rendered: Rendered, file: str, checker: Checker, limits):
    def sp(key):
        return rendered.get(key[0], key[1])

    kw = dict(q.kw)
    chain = kw.pop("chain", ())
    kw_make = {}
    token_kind = kw.pop("token_kind", None)
    token_text = kw.pop("token_text", None)
    prefix_from = kw.pop("prefix_from", None)
    profile = kw.pop("profile_id", None)
    primary = sp(q.primary)
    if token_kind:
        kw_make["unexpected_token"] = (token_kind, token_text, primary)
    if prefix_from is not None:
        kw_make["recognized_prefix"] = (sp(prefix_from)[0], primary[0])
    if profile:
        kw_make["profile_id"] = profile
    data = dict(q.data)
    if isinstance(data.get("contract_origin"), dict):
        data["contract_origin"] = dict(data["contract_origin"], file=file)
    return diagnostics.make(
        q.code, q.sub, file=file, primary=primary,
        related=[(role, sp(k)) for role, k in q.related], data=data,
        cause_chain=[(c[0], c[1], sp(c[2]), c[3]) for c in chain], **kw, **kw_make)


# ============================================================ evaluator (K sections 3, 4.4, 8.2)

DEFAULT_LIMITS = {"string_bytes": 65536, "live_text_bytes": 262144, "text_allocations": 4096}


class Sv:
    """A String value: validated UTF-8 bytes plus accounting state."""

    __slots__ = ("b", "owned", "freed")

    def __init__(self, b: bytes):
        self.b = b
        self.owned = False
        self.freed = False


class Opt:
    def __init__(self, some, v=None):
        self.some = some
        self.v = v


class Res:
    def __init__(self, ok, v):
        self.ok = ok
        self.v = v


class StructV:
    def __init__(self, name, fields):
        self.name = name
        self.fields = fields      # ordered dict name -> value


class EnumV:
    def __init__(self, name, variant, payload):
        self.name = name
        self.variant = variant
        self.payload = payload


class ResV:
    def __init__(self, slot, n):
        self.slot = slot
        self.n = n
        self.closed = False


class Fault(Exception):
    def __init__(self, code, key, data):
        self.code = code
        self.key = key
        self.data = data


class Return(Exception):
    def __init__(self, value):
        self.value = value


class Cell:
    def __init__(self, value):
        self.value = value
        self.consumed = False


class Outcome:
    def __init__(self):
        self.stdout: list = []
        self.events: list = []
        self.fault = None
        self.value = None
        self.main_kind = "unit"


def values_eq(a, b) -> bool:
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if isinstance(a, int) and isinstance(b, int):
        return a == b
    if isinstance(a, Sv) and isinstance(b, Sv):
        return string_cmp(a.b, b.b) == 0
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(values_eq(x, y) for x, y in zip(a, b))
    if isinstance(a, Opt) and isinstance(b, Opt):
        return a.some == b.some and (not a.some or values_eq(a.v, b.v))
    if isinstance(a, Res) and isinstance(b, Res):
        return a.ok == b.ok and values_eq(a.v, b.v)
    if isinstance(a, StructV) and isinstance(b, StructV):
        return a.name == b.name and all(values_eq(a.fields[k], b.fields[k]) for k in a.fields)
    if isinstance(a, EnumV) and isinstance(b, EnumV):
        return (a.name == b.name and a.variant == b.variant and len(a.payload) == len(b.payload)
                and all(values_eq(x, y) for x, y in zip(a.payload, b.payload)))
    if a == () and b == ():
        return True
    return False


def format_value(v) -> bytes:
    """Presentation of K 3.1 / A2 12.4 (strings raw, Array ``[a, b]``, Result/Option wrapped)."""
    if isinstance(v, bool):
        return b"true" if v else b"false"
    if isinstance(v, int):
        return from_i64(v)
    if isinstance(v, Sv):
        return v.b
    if v == ():
        return b"()"
    if isinstance(v, list):
        return b"[" + b", ".join(format_value(x) for x in v) + b"]"
    if isinstance(v, Opt):
        return b"Some(" + format_value(v.v) + b")" if v.some else b"None"
    if isinstance(v, Res):
        return (b"Ok(" if v.ok else b"Err(") + format_value(v.v) + b")"
    if isinstance(v, StructV):
        inner = b", ".join(utf8ref.text(k) + b": " + format_value(x) for k, x in v.fields.items())
        return utf8ref.text(v.name) + b"(" + inner + b")"
    if isinstance(v, EnumV):
        head = utf8ref.text(v.name + "." + v.variant)
        if v.payload:
            return head + b"(" + b", ".join(format_value(x) for x in v.payload) + b")"
        return head
    raise TypeError(v)


class Interp:
    def __init__(self, prog: Program, rendered: Rendered, limits=None, profile_id=None):
        self.prog = prog
        self.r = rendered
        self.limits = dict(DEFAULT_LIMITS)
        self.limits.update(limits or {})
        self.profile_id = profile_id
        self.out = Outcome()
        self.funcs = {d.name: d for d in prog.decls if isinstance(d, Fn)}
        self.structs = {d.name: d for d in prog.decls if isinstance(d, Struct)}
        self.enums = {d.name: d for d in prog.decls if isinstance(d, Enum)}
        self.methods = {}
        for d in prog.decls:
            if isinstance(d, Extend):
                for m in d.methods:
                    self.methods[(d.ty_name, m.name)] = m
        self.live = 0
        self.allocs = 0
        self.fresh: list = []
        self.slot = 0
        self.scopes: list = []

    # ---------------------------------------------------- accounting
    def new_str(self, b: bytes, key) -> Sv:
        lim = self.limits
        if len(b) > lim["string_bytes"]:
            raise Fault("R0002", key, {"runtime_budget_kind": "string_bytes",
                                       "limit": lim["string_bytes"], "observed": len(b)})
        if self.live + len(b) > lim["live_text_bytes"]:
            raise Fault("R0002", key, {"runtime_budget_kind": "live_text_bytes",
                                       "limit": lim["live_text_bytes"], "observed": self.live + len(b)})
        if self.allocs + 1 > lim["text_allocations"]:
            raise Fault("R0002", key, {"runtime_budget_kind": "text_allocations",
                                       "limit": lim["text_allocations"], "observed": self.allocs + 1})
        self.allocs += 1
        self.live += len(b)
        sv = Sv(b)
        self.fresh.append(sv)
        return sv

    def own_value(self, v):
        if isinstance(v, Sv):
            v.owned = True
        elif isinstance(v, list):
            for x in v:
                self.own_value(x)
        elif isinstance(v, Opt) and v.some:
            self.own_value(v.v)
        elif isinstance(v, Res):
            self.own_value(v.v)
        elif isinstance(v, StructV):
            for x in v.fields.values():
                self.own_value(x)
        elif isinstance(v, EnumV):
            for x in v.payload:
                self.own_value(x)

    def free_value(self, v):
        if isinstance(v, Sv):
            if not v.freed:
                v.freed = True
                self.live -= len(v.b)
        elif isinstance(v, list):
            for x in v:
                self.free_value(x)
        elif isinstance(v, Opt) and v.some:
            self.free_value(v.v)
        elif isinstance(v, Res):
            self.free_value(v.v)
        elif isinstance(v, StructV):
            for x in v.fields.values():
                self.free_value(x)
        elif isinstance(v, EnumV):
            for x in v.payload:
                self.free_value(x)

    def end_statement(self):
        for sv in self.fresh:
            if not sv.owned and not sv.freed:
                sv.freed = True
                self.live -= len(sv.b)
        self.fresh = []

    # ---------------------------------------------------- resources
    def close_value(self, v):
        if isinstance(v, ResV):
            if not v.closed:
                v.closed = True
                self.out.events.append(f"close:{v.slot}:{v.n}")
        elif isinstance(v, StructV):
            for x in reversed(list(v.fields.values())):
                self.close_value(x)
        elif isinstance(v, EnumV):
            for x in reversed(v.payload):
                self.close_value(x)
        elif isinstance(v, list):
            for x in reversed(v):
                self.close_value(x)
        elif isinstance(v, Opt) and v.some:
            self.close_value(v.v)
        elif isinstance(v, Res):
            self.close_value(v.v)

    # ---------------------------------------------------- copy
    def copy_value(self, v, key):
        if isinstance(v, Sv):
            return self.new_str(v.b, key)
        if isinstance(v, list):
            return [self.copy_value(x, key) for x in v]
        if isinstance(v, Opt):
            return Opt(v.some, self.copy_value(v.v, key) if v.some else None)
        if isinstance(v, Res):
            return Res(v.ok, self.copy_value(v.v, key))
        if isinstance(v, StructV):
            return StructV(v.name, {k: self.copy_value(x, key) for k, x in v.fields.items()})
        if isinstance(v, EnumV):
            return EnumV(v.name, v.variant, [self.copy_value(x, key) for x in v.payload])
        return v

    def is_place(self, e) -> bool:
        return is_place(e)

    def eval_owner(self, e, scope):
        """Expression in owning context: a place is copied, a temporary is moved."""
        if isinstance(e, Mv):
            return self.take(e.expr, scope)
        v = self.eval(e, scope)
        if self.is_place(e):
            return self.copy_value(v, (e, "all"))
        return v

    def take(self, e, scope):
        if isinstance(e, Name):
            cell = scope[e.name]
            cell.consumed = True
            return cell.value
        return self.eval(e, scope)

    # ---------------------------------------------------- execution
    def run(self) -> Outcome:
        main = self.funcs.get("Main")
        try:
            result = self.call_fn(main, [], {})
            if main.ret is not None and main.ret.name == "Result":
                self.out.main_kind = "ok" if isinstance(result, Res) and result.ok else "err"
                self.out.value = result
        except Fault as f:
            self.out.fault = f
        return self.out

    def call_fn(self, f: Fn, args, refs, self_value=None):
        scope = {}
        for p, a in zip(f.params, args):
            scope[p.name] = a if isinstance(a, Cell) else Cell(a)
        try:
            self.exec_block(f.body, scope, owned_scope=scope)
        except Return as r:
            return r.value
        return ()

    def exec_block(self, stmts, scope, owned_scope):
        local = dict(scope)
        declared = []
        try:
            for st in stmts:
                self.exec_stmt(st, local, declared)
                self.end_statement()
        finally:
            for name in reversed(declared):
                cell = local[name]
                if not cell.consumed:
                    self.close_value(cell.value)
                    self.free_value(cell.value)
                    cell.consumed = True

    def exec_stmt(self, st, scope, declared):
        if isinstance(st, Let):
            v = self.eval_owner(st.init, scope)
            self.own_value(v)
            scope[st.name] = Cell(v)
            declared.append(st.name)
        elif isinstance(st, Assign):
            self.assign(st, scope)
        elif isinstance(st, ExprS):
            self.eval(st.expr, scope)
        elif isinstance(st, Ret):
            v = self.eval_owner(st.expr, scope)
            raise Return(v)
        elif isinstance(st, For):
            it = self.eval(st.iter, scope)
            for item in list(it):
                inner = dict(scope)
                inner[st.var] = Cell(item)
                self.exec_block(st.body, inner, inner)
                self.end_statement()
        elif isinstance(st, MatchS):
            self.exec_match(st, scope)
        else:
            raise TypeError(st)

    def lvalue(self, e, scope):
        if isinstance(e, Name):
            cell = scope[e.name]
            return (lambda: cell.value), (lambda v: setattr(cell, "value", v))
        if isinstance(e, Field_):
            base = self.eval(e.base, scope)
            return (lambda: base.fields[e.name]), (lambda v: base.fields.__setitem__(e.name, v))
        if isinstance(e, Idx):
            base = self.eval(e.base, scope)
            i = self.eval(e.index, scope)
            return (lambda: base[i]), (lambda v: base.__setitem__(i, v))
        raise TypeError(e)

    def assign(self, st: Assign, scope):
        get, put = self.lvalue(st.target, scope)
        if st.op == "=":
            new = self.eval_owner(st.expr, scope)
            self.own_value(new)
            old = get()
            put(new)
            self.close_value(old)
            self.free_value(old)
            return
        rhs = self.eval(st.expr, scope)
        cur = get()
        put({"+=": cur + rhs, "-=": cur - rhs, "*=": cur * rhs}[st.op])

    def exec_match(self, st: MatchS, scope):
        v = self.take(st.scrut, scope) if st.move else self.eval(st.scrut, scope)
        for arm in st.arms:
            binds = {}
            if self.matches(arm.pattern, v, binds):
                inner = dict(scope)
                for k, val in binds.items():
                    inner[k] = Cell(val)
                declared = [k for k in binds]
                try:
                    self.exec_block(arm.body, inner, inner)
                finally:
                    for name in reversed(declared):
                        cell = inner[name]
                        if not cell.consumed and st.move:
                            self.close_value(cell.value)
                            cell.consumed = True
                return
        raise Fault("R0301", (st, "all"), {})

    def matches(self, p, v, binds) -> bool:
        if isinstance(p, PWild):
            return True
        if isinstance(p, PBind):
            binds[p.name] = v
            return True
        if isinstance(p, PNone):
            return isinstance(v, Opt) and not v.some
        if isinstance(p, PSome):
            return isinstance(v, Opt) and v.some and self.matches(p.sub, v.v, binds)
        if isinstance(p, PVar):
            if not isinstance(v, EnumV) or v.variant != p.variant:
                return False
            subs = p.subs or []
            return len(subs) == len(v.payload) and all(
                self.matches(s, x, binds) for s, x in zip(subs, v.payload))
        raise TypeError(p)

    # ---------------------------------------------------- expressions
    def eval(self, e, scope):
        if isinstance(e, Lit):
            return e.value
        if isinstance(e, Name):
            if e.name == "None":
                return Opt(False)
            return scope[e.name].value
        if isinstance(e, Mv):
            return self.take(e.expr, scope)
        if isinstance(e, Raw):
            return ()
        if isinstance(e, Str):
            return self.eval_str(e, scope)
        if isinstance(e, Arr):
            return [self.eval_owner(i, scope) for i in e.items]
        if isinstance(e, Field_):
            return self.eval(e.base, scope).fields[e.name]
        if isinstance(e, Idx):
            return self.eval(e.base, scope)[self.eval(e.index, scope)]
        if isinstance(e, Bin):
            return self.eval_bin(e, scope)
        if isinstance(e, IfE):
            return self.eval(e.a if self.eval(e.cond, scope) else e.b, scope)
        if isinstance(e, Try):
            v = self.eval(e.expr, scope)
            if isinstance(v, Res) and not v.ok:
                raise Return(v)
            return v.v
        if isinstance(e, Var):
            args = [self.eval_owner(a.expr, scope) for a in (e.args or [])]
            return EnumV(e.ty_name, e.variant, args)
        if isinstance(e, MCall):
            return self.eval_mcall(e, scope)
        if isinstance(e, Call):
            return self.eval_call(e, scope)
        raise TypeError(e)

    def eval_str(self, e: Str, scope):
        buf = b""
        for part in e.parts:
            if isinstance(part, str):
                buf += utf8ref.text(part)
            else:
                buf += format_value(self.eval(part, scope))
        return self.new_str(buf, (e, "all"))

    def eval_bin(self, e: Bin, scope):
        a = self.eval(e.left, scope)
        b = self.eval(e.right, scope)
        op = e.op
        if isinstance(a, Sv) and isinstance(b, Sv):
            return compare_op(op, a.b, b.b)
        if op == "==":
            return values_eq(a, b)
        if op == "!=":
            return not values_eq(a, b)
        if op == "<":
            return a < b
        if op == "<=":
            return a <= b
        if op == ">":
            return a > b
        if op == ">=":
            return a >= b
        if op == "+":
            return a + b
        if op == "-":
            return a - b
        if op == "*":
            return a * b
        raise ValueError(op)

    def eval_mcall(self, e: MCall, scope):
        recv = self.eval(e.recv, scope)
        args = [self.eval(a.expr, scope) for a in e.args]
        if isinstance(recv, Sv):
            b = recv.b
            key = (e, "all")
            if e.name == "ByteLen":
                return len(b)
            if e.name == "ScalarLen":
                return utf8ref.scalar_len(b)
            if e.name == "IsEmpty":
                return len(b) == 0
            if e.name == "Concat":
                return self.new_str(b + args[0].b, key)
            if e.name == "SliceBytes":
                tag, val = slice_bytes(b, args[0], args[1])
                if tag == "Ok":
                    return Res(True, self.new_str(val, key))
                return Res(False, val)
            if e.name == "Scalars":
                return list(scalars_of(b))
            raise ValueError(e.name)
        if isinstance(recv, (StructV, EnumV)):
            m = self.methods[(recv.name, e.name)]
            return self.call_fn(m, [Cell(recv)] + [self.eval_owner(a.expr, scope) for a in e.args], {})
        raise ValueError(e.name)

    def eval_call(self, e: Call, scope):
        name = e.callee
        if name in self.structs:
            d = self.structs[name]
            assigned = {}
            prepared = []
            positional = [a for a in e.args if a.label is None]
            order = positional + [a for a in e.args if a.label is not None]
            try:
                for a in order:
                    v = self.eval_owner(a.expr, scope)
                    prepared.append(v)
                    if a.label is not None:
                        assigned[a.label] = v
                    else:
                        assigned[d.fields[positional.index(a)].name] = v
            except Fault:
                for v in reversed(prepared):
                    self.close_value(v)
                raise
            return StructV(name, {f.name: assigned[f.name] for f in d.fields})
        if name == "Print":
            v = self.eval(e.args[0].expr, scope)
            self.out.stdout.append(format_value(v))
            return ()
        if name == "AssertEq":
            a = self.eval(e.args[0].expr, scope)
            b = self.eval(e.args[1].expr, scope)
            if not values_eq(a, b):
                raise Fault("R0201", (e, "all"), {})
            return ()
        if name == "Push":
            arr = self.eval(e.args[0].expr, scope)
            v = self.eval_owner(e.args[1].expr, scope)
            self.own_value(v)
            arr.append(v)
            return ()
        if name == "Pop":
            arr = self.eval(e.args[0].expr, scope)
            if not arr:
                return Opt(False)
            v = arr.pop()
            self.disown(v)
            return Opt(True, v)
        if name == "Len":
            return len(self.eval(e.args[0].expr, scope))
        if name == "JoinArrays":
            a = self.eval(e.args[0].expr, scope)
            b = self.eval(e.args[1].expr, scope)
            key = (e, "all")
            return [self.copy_value(x, key) for x in a] + [self.copy_value(x, key) for x in b]
        if name == "Some":
            return Opt(True, self.eval_owner(e.args[0].expr, scope))
        if name == "Ok":
            return Res(True, self.eval_owner(e.args[0].expr, scope))
        if name == "Err":
            return Res(False, self.eval_owner(e.args[0].expr, scope))
        if name in ("std.parse.I64", "parse.I64"):
            tag, val = parse_i64(self.eval(e.args[0].expr, scope).b)
            return Res(tag == "Ok", val)
        if name in ("std.text.FromI64", "text.FromI64"):
            return self.new_str(from_i64(self.eval(e.args[0].expr, scope)), (e, "all"))
        if name == "MockOpen":
            n = self.eval(e.args[0].expr, scope)
            r = ResV(self.slot, n)
            self.slot += 1
            self.out.events.append(f"open:{r.slot}:{n}")
            return r
        if name == "Inspect":
            r = self.eval(e.args[0].expr, scope)
            self.out.events.append(f"inspect:{r.slot}:{r.n}")
            return ()
        if name == "Fail":
            raise Fault("R0301", (e, "all"), {})
        if name == "Drop":
            a = e.args[0].expr
            v = self.take(a, scope)
            self.close_value(v)
            self.free_value(v)
            return ()
        f = self.funcs[name]
        args = []
        for p, a in zip(f.params, e.args):
            if p.mode == "edit":
                args.append(self.cell_of(a.expr, scope))
            elif p.mode == "view":
                args.append(self.eval(a.expr, scope))
            else:
                args.append(self.eval_owner(a.expr, scope))
        return self.call_fn(f, args, {})

    def cell_of(self, e, scope):
        if isinstance(e, Name):
            return scope[e.name]
        get, put = self.lvalue(e, scope)
        holder = Cell(get())
        return holder

    def disown(self, v):
        if isinstance(v, Sv):
            v.owned = False
            self.fresh.append(v)
        elif isinstance(v, list):
            for x in v:
                self.disown(x)
        elif isinstance(v, StructV):
            for x in v.fields.values():
                self.disown(x)
        elif isinstance(v, EnumV):
            for x in v.payload:
                self.disown(x)


def run_program(prog: Program, file: str, limits=None, profile_id=None):
    """Render and evaluate; returns (source, Outcome, fault diagnostic or None)."""
    rendered = render(prog)
    interp = Interp(prog, rendered, limits, profile_id)
    out = interp.run()
    fault = None
    if out.fault is not None and out.fault.code == "R0002":
        kw = {"profile_id": profile_id} if profile_id else {}
        fault = diagnostics.make("R0002", file=file, primary=rendered.get(*out.fault.key),
                                 data=out.fault.data, **kw)
    return rendered.source, out, fault


def check_import_access(defs: Program, def_file: str, use_prog: Program, use_file: str):
    """Access test of the interface resolver (K 5.3, P04): a private declaration cannot be
    imported from a requesting domain outside the defining one."""
    dr = render(defs)
    ur = render(use_prog)
    use = next(d for d in use_prog.decls if isinstance(d, Use))
    name = use.path.split(".")[-1]
    decl = next((d for d in defs.decls if isinstance(d, (Struct, Enum)) and d.name == name), None)
    if decl is not None and not decl.pub:
        return [diagnostics.make("NX-E1308", "private_access", file=use_file,
                                 primary=ur.get(use, "last"),
                                 related=[("declaration", dr.get(decl, "name"), def_file)])]
    return []
