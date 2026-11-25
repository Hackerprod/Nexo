#!/usr/bin/env python3
"""Oraculo independiente de I07c-S5c (union `if` con ancla pendiente, aplanado, asignacion posterior).

Deriva SOLO del ADR (docs/adr/F0-0002-second-slice.md 4.1, 4.1.1, 4.1.2 clausulas 1-7, 4.7, 13.9, y el addendum
docs/adr/F0-0002-lagunas.md). No importa, lee ni ejecuta el verificador. Solo biblioteca estandar.

Uso:
  python -B scripts/i07c_s5c_oraculo.py FUENTE.n            JSON por programa (diagnosticos esperados)
  python -B scripts/i07c_s5c_oraculo.py --source 'fn Main(){...}'
  python -B scripts/i07c_s5c_oraculo.py --batch FUENTES.json  {id: fuente} o [{id, source}] -> {id: resultado}'
  python -B scripts/i07c_s5c_oraculo.py --matrix [--md RUTA]  tabla de las filas de la matriz (fuentes recuperadas)
  python -B scripts/i07c_s5c_oraculo.py --self-test         ejemplos literales del ADR

Modelo (cada regla cita su clausula; "L" = addendum de lagunas):
 * Tipos: nodos con union-find; igualdad ATOMICA (cl.1: sin unificaciones parciales al fallar) y occurs-check
   (recursiva = incompatible: cl.2, L "no autoriza alpha=Array<alpha> con r").
 * Orden de comprobacion fuente (cl.1): condicion antes de ramas; contexto antes de hijos; hijos en orden.
 * Union sin contexto externo (cl.4, 4.7, L): ancla = primera continuacion con evidencia valida; resto aplanado en
   orden fuente (incluidas las primeras continuaciones de if anidados posteriores); UNA sola NX-E1303 en la primera
   incompatible; las demas absorbidas. Un if anidado en la primera continuacion tiene union propia y aporta su ancla
   como testigo parcial (cl.4, cl.5). Relacion expected_type = operando de la primera continuacion (cl.6, l.371).
 * Conflicto con tipos pendientes: se conserva y se completan SUS tipos con igualdades compatibles posteriores
   (cl.2); si al cierre sigue pendiente se omite (el constructor emite NX-E0103) y CUENTA como denunciado (cl.5, l.367).
 * Asignacion posterior: se comprueba contra el tipo ya fijado, sin redistribuir retrospectivamente (cl.4 "Disponibilidad
   por nodo", cl.7, L); incompatible => NX-E0102 en la hoja del RHS; asignacion rechazada no cambia el tipo (cl.2).
 * Orden de la lista: 13.9 "se ordenan por file, primary_span.start, primary_span.end, code y subcause" (null = "").
"""
import json, os, re, sys
from pathlib import Path

# ----------------------------------------------------------------------------------------------------------------
# Lexer / parser del subconjunto
# ----------------------------------------------------------------------------------------------------------------
KW = {"fn", "let", "var", "if", "then", "else", "value", "return", "true", "false", "while"}
TOK = re.compile(r"\s+|//[^\n]*|(?P<id>[A-Za-z_][A-Za-z0-9_]*)|(?P<int>[0-9]+)|(?P<sym>==|[=<+\-()\[\]{},;:])")


class Syntax(Exception):
    pass


def lex(src):
    out, i = [], 0
    while i < len(src):
        m = TOK.match(src, i)
        if not m:
            raise Syntax("lex@%d" % i)
        if m.lastgroup:
            out.append((m.lastgroup, m.group(), m.start(), m.end()))
        i = m.end()
    out.append(("eof", "", len(src), len(src)))
    return out


class N:
    def __init__(self, k, s, e, **kw):
        self.k, self.s, self.e = k, s, e
        self.__dict__.update(kw)


class Parser:
    def __init__(self, src):
        self.t, self.p = lex(src), 0

    def peek(self, o=0):
        return self.t[min(self.p + o, len(self.t) - 1)]

    def eat(self, v=None):
        tk = self.t[self.p]
        if v is not None and tk[1] != v:
            raise Syntax("esperaba %r vi %r @%d" % (v, tk[1], tk[2]))
        self.p += 1
        return tk

    def is_(self, v, o=0):
        return self.peek(o)[1] == v and self.peek(o)[0] != "int"

    def program(self):
        fns = []
        while self.peek()[0] != "eof":
            self.eat("fn")
            name = self.eat()[1]
            self.eat("(")
            self.eat(")")
            self.eat("{")
            body = []
            while not self.is_("}"):
                body.append(self.stmt())
            self.eat("}")
            fns.append((name, body))
        return fns

    def ty(self):
        tk = self.eat()
        if tk[1] == "Array":
            self.eat("<")
            raise Syntax("anotacion Array no modelada")
        return N("tyname", tk[2], tk[3], name=tk[1])

    def stmt(self):
        tk = self.peek()
        if tk[1] in ("let", "var") and tk[0] == "id":
            self.eat()
            name = self.eat()[1]
            ann = None
            if self.is_(":"):
                self.eat()
                ann = self.ty()
            self.eat("=")
            e = self.expr()
            end = self.eat(";")[3]
            return N("bind", tk[2], end, mut=(tk[1] == "var"), name=name, ann=ann, init=e)
        if tk[0] == "id" and tk[1] not in KW and self.is_("=", 1):
            name = self.eat()[1]
            self.eat("=")
            e = self.expr()
            end = self.eat(";")[3]
            return N("assign", tk[2], end, name=name, rhs=e)
        if tk[1] == "return":
            self.eat()
            end = self.eat(";")[3]
            return N("retstmt", tk[2], end)
        e = self.expr()
        end = self.eat(";")[3]
        return N("exprstmt", tk[2], end, ex=e)

    def expr(self):
        if self.is_("if"):
            return self.ifexpr()
        return self.cmp()

    def ifexpr(self):
        s = self.eat("if")[2]
        c = self.expr()
        self.eat("then")
        a = self.expr()
        self.eat("else")
        b = self.expr()
        return N("if", s, b.e, cond=c, then=a, els=b)

    def cmp(self):
        l = self.add()
        if self.peek()[1] in ("==", "<") and self.peek()[0] == "sym":
            op = self.eat()[1]
            r = self.add()
            return N("bin", l.s, r.e, op=op, l=l, r=r)
        return l

    def add(self):
        l = self.post()
        while self.peek()[1] in ("+", "-") and self.peek()[0] == "sym":
            op = self.eat()[1]
            r = self.post()
            l = N("bin", l.s, r.e, op=op, l=l, r=r)
        return l

    def post(self):
        p = self.prim()
        while self.is_("["):
            self.eat("[")
            i = self.expr()
            end = self.eat("]")[3]
            p = N("index", p.s, end, base=p, idx=i)
        return p

    def prim(self):
        tk = self.peek()
        if tk[0] == "int":
            self.eat()
            return N("int", tk[2], tk[3])
        if tk[1] in ("true", "false") and tk[0] == "id":
            self.eat()
            return N("bool", tk[2], tk[3])
        if tk[1] == "if" and tk[0] == "id":
            return self.ifexpr()
        if tk[0] == "id" and tk[1] not in KW:
            if self.is_("(", 1):
                self.eat()
                self.eat("(")
                args = []
                while not self.is_(")"):
                    args.append(self.expr())
                    if self.is_(","):
                        self.eat()
                end = self.eat(")")[3]
                return N("call", tk[2], end, name=tk[1], args=args)
            self.eat()
            return N("var", tk[2], tk[3], name=tk[1])
        if tk[1] == "(":
            self.eat()
            e = self.expr()
            end = self.eat(")")[3]
            return N("group", tk[2], end, inner=e)
        if tk[1] == "[":
            self.eat()
            els = []
            while not self.is_("]"):
                els.append(self.expr())
                if self.is_(","):
                    self.eat()
            end = self.eat("]")[3]
            return N("arr", tk[2], end, elems=els)
        if tk[1] == "{":
            self.eat()
            if self.is_("return"):
                self.eat()
                self.eat(";")
                end = self.eat("}")[3]
                return N("never", tk[2], end)
            self.eat("value")
            self.eat(":")
            e = self.expr()
            self.eat(";")
            end = self.eat("}")[3]
            return N("block", tk[2], end, value=e)
        raise Syntax("primario inesperado %r @%d" % (tk[1], tk[2]))


def strip(e):
    """cl.1 y addendum: los parentesis de agrupacion nunca amplian el primario; bloques con valor conservan la
    ocurrencia del valor (cl.1)."""
    while e.k in ("group", "block"):
        e = e.inner if e.k == "group" else e.value
    return e


def span(e):
    e = strip(e)
    return (e.s, e.e)


# ----------------------------------------------------------------------------------------------------------------
# Tipos
# ----------------------------------------------------------------------------------------------------------------
class T:
    __slots__ = ("k", "el", "ref", "wit", "src", "id")
    _n = 0

    def __init__(self, k, el=None):
        self.k, self.el, self.ref, self.wit, self.src = k, el, None, None, None
        T._n += 1
        self.id = T._n


def var():
    return T("var")


def arr(el):
    return T("arr", el)


def prune(t):
    while t.k == "var" and t.ref is not None:
        t = t.ref
    return t


def occurs(v, t):
    t = prune(t)
    return t is v or (t.k == "arr" and occurs(v, t.el))


def unbound(t):
    return prune(t).k == "var"


def has_var(t):
    t = prune(t)
    return t.k == "var" or (t.k == "arr" and has_var(t.el))


def show(t):
    t = prune(t)
    if t.k == "i64":
        return "I64"
    if t.k == "bool":
        return "Bool"
    if t.k == "arr":
        return "Array<%s>" % show(t.el)
    return "?"


def src_of_type(t, dived=None):
    """Origen concreto de un tipo esperado (cl.6/l.371: siguiendo aliases hasta el inicializador o la restriccion
    posterior que lo fijo). Una variable ligada conserva el operando de su PRIMERA igualdad (testigo)."""
    seen = 0
    while t is not None and seen < 64:
        seen += 1
        if t.k == "var":
            if t.ref is None:
                return None
            if t.ref.k != "var" and t.wit is not None:
                return t.wit  # la igualdad que completo la variable con estructura concreta
            t = t.ref  # alias variable->variable: se sigue hasta la restriccion que la fijo
            continue
        if t.src is not None:
            return t.src
        if t.k == "arr":
            if dived is not None:
                dived.append(1)  # el tipo compuesto no tiene origen propio: se desciende al elemento
            t = t.el
            continue
        return None
    return None


class Unifier:
    def __init__(self):
        self.trail = []

    def _u(self, a, b, sa, sb):
        a, b = prune(a), prune(b)
        if a is b:
            return True
        if a.k == "var":
            if occurs(a, b):
                return False
            a.ref, a.wit = b, sb  # a se completa con estructura aportada por el operando b
            self.trail.append(a)
            return True
        if b.k == "var":
            if occurs(b, a):
                return False
            b.ref, b.wit = a, sa
            self.trail.append(b)
            return True
        if a.k != b.k:
            return False
        if a.k == "arr":
            return self._u(a.el, b.el, sa, sb)
        return True

    def unify(self, a, b, sa=None, sb=None):
        """cl.1: igualdad atomica; los fallos no dejan unificaciones parciales."""
        mark = len(self.trail)
        if self._u(a, b, sa, sb):
            return True
        for v in self.trail[mark:]:
            v.ref = None
            v.wit = None
        del self.trail[mark:]
        return False


# ----------------------------------------------------------------------------------------------------------------
# Comprobador
# ----------------------------------------------------------------------------------------------------------------
I64, BOOL = "i64", "bool"


class Val:
    """ty: nodo de tipo valido | None si sin evidencia (error previo, contaminado o Never)."""

    def __init__(self, ty=None, never=False, bad=False, anchor=None):
        self.ty, self.never, self.bad, self.anchor = ty, never, bad, anchor


class Ctx:
    """Contexto esperado. mode: 'ext' (anotacion/condicion/selector/operando: cada hoja incompatible es C),
    'ref' (referencia de homogeneidad de un array), 'assign' (tipo del destino de una asignacion).
    rel: span fijo, None (intrinseco: sin relacion expected_type) o 'origin' (origen concreto del esperado).
    wit: operando que se registra como testigo al fijar una variable (asignacion: el RHS completo, l.309).
    box: estado compartido de supresion/consumo de la denuncia de dominio (L "Operandos de +/-") o D de 4.1.2.5."""

    def __init__(self, ty, mode, rel, wit=None, box=None):
        self.ty, self.mode, self.rel, self.wit, self.box = ty, mode, rel, wit, box
        self.lit = False  # contexto distribuido a los elementos de un literal de array (marca de decision no fijada)

    def sub(self, ty):
        return Ctx(ty, self.mode, self.rel, self.wit, self.box)


class Conflict:
    def __init__(self, code, sub, prim, exp, found, rel, relkind, cls="C", why=""):
        self.code, self.sub, self.prim, self.exp, self.found = code, sub, prim, exp, found
        self.rel, self.relkind, self.cls, self.why = rel, relkind, cls, why


class Checker:
    def __init__(self, src):
        self.src = src
        self.u = Unifier()
        self.env = {}  # nombre -> dict(ty, mut, tainted)
        self.conflicts = []
        self.ctors = []  # (span, elem)  -> NX-E0103 al cierre si elem sigue variable (cl.2, 4.1)
        self.projs = []  # (base, result): "si la base es Array, el resultado es su elemento" (cl.3)
        self.notes = []
        self.unmodeled = []
        self.nonfixed = set()  # decisiones que el ADR no fija y que el oraculo toma con la lectura mas cercana

    # -- utilidades ---------------------------------------------------------------------------------------------
    def unify(self, a, b, sa=None, sb=None):
        ok = self.u.unify(a, b, sa, sb)
        if ok:
            self.propagate()
        return ok

    def propagate(self):
        """Relacion condicional de proyeccion (cl.3): al establecerse el constructor Array de la base, el resultado
        pasa a ser su elemento, antes de comprobar las hojas de restricciones posteriores."""
        changed = True
        while changed:
            changed = False
            for base, res in self.projs:
                b = prune(base)
                if b.k == "arr" and not getattr(res, "_done", False):
                    r = prune(res)
                    if not (r is prune(b.el)):
                        if self.u.unify(res, b.el, None, None):
                            changed = True
                    try:
                        res._done = True
                    except AttributeError:
                        pass

    def conflict(self, **kw):
        self.conflicts.append(Conflict(**kw))

    # -- hojas --------------------------------------------------------------------------------------------------
    def leaf(self, e, t, ctx):
        """Contrasta el tipo de una hoja con el contexto esperado (cl.4: la hoja incompatible con un esperado
        concreto impuesto desde fuera es C propio, NX-E0102/type_mismatch, en su propia ocurrencia)."""
        if t is None:
            return False
        sp = span(e)
        w = ctx.wit if ctx.wit is not None else sp
        other = ctx.rel if isinstance(ctx.rel, tuple) else None
        mark = len(self.u.trail)
        if self.unify(ctx.ty, t, other, w):
            if ctx.lit and ctx.wit is None and len(self.u.trail) > mark:
                # el ADR no fija si el testigo de una variable fijada por la igualdad de un elemento de literal de
                # array (referencia de homogeneidad) es el elemento hoja o el elemento completo (cl.1 "contexto
                # distribuido a cada elemento" frente a l.371 "operando"): se elige la hoja
                self.nonfixed.add("T1 testigo de variable fijada en elemento de literal")
            return True
        box = ctx.box
        if box is not None and box.get("suppress"):
            return False  # La igualdad rechazada no aporta tipo, aunque el diagnóstico se suprima.
        cls = "C"
        if ctx.mode == "ref" and box is not None:
            # cl.5: dentro de una misma restriccion causal con referencia derivada, la primera es C y las
            # adicionales D (presencia I: se listan aparte, no cuentan en la comparacion estricta)
            if box.get("first_done"):
                cls = "D"
            box["first_done"] = True
        if box is not None and ctx.mode == "ext" and box.get("consume"):
            box["consumed"] = True
        self.conflict(code="NX-E0102", sub="type_mismatch", prim=sp, exp=ctx.ty, found=t, rel=ctx.rel,
                      relkind="origin" if ctx.rel == "origin" else "fixed", cls=cls)
        return False

    # -- expresiones --------------------------------------------------------------------------------------------
    def check(self, e, ctx=None):
        g = strip(e)
        k = g.k
        if k == "never":
            return Val(never=True)
        if k == "if":
            return self.check_if(g, ctx)
        if k == "arr":
            return self.check_arr(g, ctx)
        v = self.synth_simple(g)
        if ctx is not None and v.ty is not None:
            if not self.leaf(g, v.ty, ctx):
                return Val(v.ty, bad=True)  # Tipo interno disponible, resultado inválido para consumidores.
        return v

    def determined(self, ctx):
        return ctx is not None and not unbound(ctx.ty)

    def synth_simple(self, g):
        k = g.k
        if k == "int":
            n = T(I64)
            return Val(n)
        if k == "bool":
            return Val(T(BOOL))
        if k == "var":
            b = self.env.get(g.name)
            if b is None:
                self.unmodeled.append("variable libre %s" % g.name)
                return Val(bad=True)
            if b["tainted"]:
                return Val(bad=True)  # cl.5: las lecturas de un binding contaminado no aportan evidencia
            return Val(b["ty"])
        if k == "index":
            return self.check_index(g)
        if k == "call":
            return self.check_call(g)
        if k == "bin":
            return self.check_bin(g)
        self.unmodeled.append("nodo " + k)
        return Val(bad=True)

    def check_call(self, g):
        if g.name == "Print":
            for a in g.args:
                self.check(a, None)
            return Val(T("unit"))
        if g.name == "Len":
            v = self.check(g.args[0], None)
            if v.ty is None or v.bad:
                return Val(T(I64)) if not v.bad else Val(bad=True)
            t = prune(v.ty)
            if t.k in (I64, BOOL):
                # cl.3: Len con argumento no Array -> E0102/not_indexable sobre el argumento
                self.conflict(code="NX-E0102", sub="not_indexable", prim=span(g.args[0]), exp="domain:array",
                              found=v.ty, rel=None, relkind="none")
                return Val(bad=True)
            return Val(T(I64))
        self.unmodeled.append("llamada " + g.name)
        return Val(bad=True)

    def check_index(self, g):
        vb = self.check(g.base, None)
        if vb.ty is None or vb.bad:
            self.check(g.idx, None)
            return Val(bad=True)
        b = prune(vb.ty)
        if b.k in (I64, BOOL):
            # cl.3: base no indexable -> not_indexable sobre la base; el selector I64 depende de la indexabilidad
            self.conflict(code="NX-E0102", sub="not_indexable", prim=span(g.base), exp="domain:array", found=vb.ty,
                          rel=None, relkind="none")
            self.check(g.idx, None)
            return Val(bad=True)
        self.check(g.idx, Ctx(T(I64), "ext", None))
        if b.k == "arr":
            return Val(b.el)
        res = var()  # base sin constructor: relacion condicional (cl.3)
        self.projs.append((vb.ty, res))
        return Val(res)

    def check_bin(self, g):
        op = g.op
        if op in ("+", "-"):
            # cl.4 (+/-): I64 de cada operando es una igualdad intrinseca determinada al entrar; una sola denuncia
            # de dominio por operacion (el primer operando que emite NX-E0102 la consume); el segundo aplica su
            # igualdad compatible sin segunda denuncia (L "Operandos de +/-").
            box = {"consume": True}
            vl = self.check(g.l, Ctx(T(I64), "ext", None, box=box))
            if box.get("consumed"):
                box["suppress"] = True
            vr = self.check(g.r, Ctx(T(I64), "ext", None, box=box))
            # cl.5: una operacion con un operando fallido o incompatible no ofrece su tipo nominal de resultado
            if box.get("consumed") or vl.bad or vr.bad:
                return Val(bad=True)
            return Val(T(I64))
        # '<' y '==': igualdad antes del dominio (cl.4)
        vl = self.check(g.l, None)
        vr = self.check(g.r, None)
        if vl.ty is None or vr.ty is None:
            # cl.5/cl.4: se suprime la igualdad cruzada con un operando contaminado y la operacion no ofrece resultado
            return Val(bad=True) if (vl.bad or vr.bad) else Val(T(BOOL))
        if not self.unify(vl.ty, vr.ty, span(g.l), span(g.r)):
            self.conflict(code="NX-E0102", sub="type_mismatch", prim=span(g.r), exp=vl.ty, found=vr.ty,
                          rel=span(g.l), relkind="fixed")
            return Val(bad=True)  # T5: el resultado nominal no sobrevive a la igualdad fallida.
        if op == "<" and prune(vl.ty).k not in (I64,) and not unbound(vl.ty):
            self.unmodeled.append("dominio de '<' con tipo no I64")
        return Val(T(BOOL))

    # -- arrays -------------------------------------------------------------------------------------------------
    def check_arr(self, g, ctx):
        sp = (g.s, g.e)
        if g.elems and ctx is not None and self.determined(ctx) and prune(ctx.ty).k == "arr":
            # cl.4: un contexto determinado exige Array -> cada elemento recibe el tipo de elemento
            E = prune(ctx.ty).el
            first = None
            any_bad = False
            for el in g.elems:
                sub_i = Ctx(E, ctx.mode, ctx.rel, ctx.wit, ctx.box)
                sub_i.lit = ctx.mode == "ref"
                v = self.check_elem(el, sub_i)
                any_bad |= v.bad
                if first is None and v is not None and v.ty is not None:
                    first = v.ty
            # el constructor responde por su elemento real: un elemento rechazado (p. ej. recursivo) no lo fija
            self.ctors.append((sp, first if first is not None else E))
            return Val(bad=True) if any_bad else Val(arr(E))
        if not g.elems:
            a = var()
            node = arr(a)
            self.ctors.append((sp, a))
            if ctx is not None:
                if not self.leaf(g, node, ctx):
                    return Val(bad=True)
            return Val(node)
        # sin contexto Array determinado: la referencia es el primer elemento no divergente con evidencia valida
        ref = None
        any_bad = False
        for el in g.elems:
            if ref is None:
                v = self.check(el, None)
                any_bad |= v.bad
                if v.ty is not None and not v.bad:
                    ref = (v.ty, span(el))
                continue
            v = self.check_elem(el, Ctx(ref[0], "ref", ref[1], box={}))
            any_bad |= v.bad
        if ref is None:
            return Val(bad=True)
        node = arr(ref[0])
        self.ctors.append((sp, ref[0]))
        if any_bad:
            return Val(bad=True)  # N: el literal con un elemento erróneo no ofrece tipo a la unión.
        if ctx is not None:
            if not self.leaf(g, node, ctx):
                return Val(bad=True)
        return Val(node)

    def check_elem(self, el, ctx):
        """Un elemento de array con contexto: raiz determinada -> distribuir; raiz variable -> se relaciona con el
        resultado sintetizado sin fingir un contexto concreto (cl.4)."""
        g = strip(el)
        if g.k in ("if", "arr") and unbound(ctx.ty):
            v = self.check(g, None)
            if v.ty is not None:
                self.leaf(g, v.ty, ctx)
            return v
        return self.check(g, ctx)

    # -- if -----------------------------------------------------------------------------------------------------
    def cond(self, c):
        return self.check(c, Ctx(T(BOOL), "ext", None, box={})).bad

    def check_if(self, g, ctx):
        bad_condition = self.cond(g.cond)
        if ctx is not None and self.determined(ctx):
            # contexto externo/referencia determinada: se distribuye a cada continuacion normal (cl.4)
            if ctx.mode == "ref":
                ctx = Ctx(ctx.ty, ctx.mode, ctx.rel, ctx.wit, ctx.box)
            then = self.check(g.then, ctx)
            other = self.check(g.els, ctx)
            return Val(bad=True) if bad_condition or then.bad or other.bad else Val(ctx.ty)
        u = self.union(g, cond_done=True)
        if bad_condition:
            return Val(bad=True)  # T5: una condición fallida no aporta testigo a la unión exterior.
        v = Val(u.ty, bad=(u.ty is None and not u.never), never=u.never, anchor=u.anchor)
        if ctx is not None and v.ty is not None:
            self.leaf(g, v.ty, ctx)  # raiz variable: se relaciona con el resultado sintetizado
        return v

    def union(self, g, cond_done=False):
        """Union `if` sin tipo esperado externo determinado (cl.4, 4.7, L)."""
        bad_condition = self.cond(g.cond) if not cond_done else False
        st = {"anchor": None, "denounced": False, "any_conflict": False, "any_bad": False, "any_normal": False}
        for branch in (g.then, g.els):
            self.cont(branch, st, first_pos=True)
        if bad_condition:
            return Val(bad=True)
        anchor = st["anchor"]
        if anchor is None:
            return Val(never=True)
        if st["any_conflict"] or st["any_bad"]:
            # cl.5: una union que discrepa o con error interno carece de tipo ante todo consumidor salvo otra
            # union (testigo parcial = su referencia)
            return Val(None, bad=True, anchor=anchor)
        return Val(anchor[0], anchor=anchor)

    def cont(self, e, st, first_pos):
        g = strip(e)
        if g.k == "never":
            return
        if st["anchor"] is None:
            # primera continuacion con evidencia valida: establece la referencia (cl.4)
            if g.k == "if":
                u = self.union(g)
                if u.anchor is not None:
                    st["anchor"] = (u.anchor[0], (g.s, g.e))  # relacion expected_type: el `if` anidado (l.371)
                    if u.ty is None:
                        st["any_bad"] = True  # la union interior discrepa: sin tipo ante la exterior? ver notas
                return
            v = self.check(g, None)
            if v.ty is not None and not v.bad:
                st["anchor"] = (v.ty, span(g))
            elif not v.never:
                st["any_bad"] = True
            return
        # continuaciones posteriores: aplanado recursivo (cl.4); no se sintetiza otra referencia dentro de ellas
        if g.k == "if":
            if self.cond(g.cond):
                self.union(g, cond_done=True)  # Las obligaciones internas de las ramas sobreviven.
                st["any_bad"] = True
                return
            self.cont(g.then, st, False)
            self.cont(g.els, st, False)
            return
        v = self.check(g, None)
        if v.ty is None or v.bad:
            if not v.never:
                st["any_bad"] = True
            return
        a_ty, a_sp = st["anchor"]
        sp = span(g)
        if self.unify(a_ty, v.ty, sb=sp, sa=a_sp):
            return
        st["any_conflict"] = True
        if st["denounced"]:
            self.notes.append(("absorbida", sp))  # cl.4: sin NX-E1303 propia (limite normativo de la union)
            return
        st["denounced"] = True  # cl.5/l.367: tambien cuenta si despues se omite por pendiente
        self.conflict(code="NX-E1303", sub=None, prim=sp, exp=a_ty, found=v.ty, rel=a_sp, relkind="fixed")

    # -- sentencias ---------------------------------------------------------------------------------------------
    def tyname(self, n):
        if n.name == "Bool":
            return T(BOOL)
        if n.name == "I64":
            return T(I64)
        raise Syntax("anotacion %s" % n.name)

    def run(self, fns):
        for name, body in fns:
            for s in body:
                self.stmt(s)
        return self.finish()

    def adopt(self, t, sp, mark):
        t = t if t.k != "var" else None
        if t is None:
            return
        # origen de un inicializador: solo nodos creados por la propia expresion y completamente concretos
        def closed(x):
            x = prune(x)
            return x.k != "var" and (x.k != "arr" or closed(x.el))

        def mk(x):
            x = prune(x)
            if x.id > mark and x.src is None and x.k != "var":
                x.src = sp
            if x.k == "arr":
                mk(x.el)

        def go(x):
            x = prune(x) if x.k == "var" and x.ref is not None else x
            if x.k == "var":
                return
            if closed(x):
                mk(x)
            elif x.k == "arr":
                go(x.el)

        go(t)

    def stmt(self, s):
        if s.k == "bind":
            mark = T._n
            ctx = None
            if s.ann is not None:
                ctx = Ctx(self.tyname(s.ann), "ext", (s.ann.s, s.ann.e))
            v = self.check(s.init, ctx)
            tainted = v.ty is None
            ty = v.ty if v.ty is not None else var()
            if ctx is not None:
                ty = ctx.ty
            ig = strip(s.init)
            if not tainted and ig.k in ("arr", "int", "bool", "bin", "call"):
                self.adopt(ty, (ig.s, ig.e), mark)
            self.env[s.name] = {"ty": ty, "mut": s.mut, "tainted": tainted and ctx is None}
        elif s.k == "assign":
            b = self.env.get(s.name)
            if b is None or b["tainted"]:
                self.check(s.rhs, None)
                return
            rhs = strip(s.rhs)
            ctx = Ctx(b["ty"], "assign", "origin", wit=(rhs.s, rhs.e))
            # una asignacion posterior se comprueba contra el tipo ya fijado (cl.2, L); no se redistribuye
            # retrospectivamente y una asignacion rechazada no cambia el tipo del destino (cl.2, cl.5)
            self.check_assign_rhs(rhs, ctx)
        elif s.k == "exprstmt":
            self.check(s.ex, None)
        elif s.k == "retstmt":
            pass

    def check_assign_rhs(self, rhs, ctx):
        self.check(rhs, ctx)

    # -- cierre -------------------------------------------------------------------------------------------------
    def finish(self):
        out = []
        # NX-E0103: constructor cuyo elemento sigue sin solucion al cierre (cl.2, 4.1); uno por constructor
        pending_ctor = []
        for sp, el in self.ctors:
            if unbound(el):
                out.append(dict(code="NX-E0103", subcause=None, primary=list(sp), expected_type="", found_type="",
                                rel_expected=None, rel_found=None, cls="C", clause="cl.2 / 4.1"))
                pending_ctor.append(sp)
        # conflictos: se completan sus tipos con las igualdades compatibles posteriores (cl.2)
        seen_sites = set()
        for c in self.conflicts:
            if c.sub == "not_indexable":
                et = c.exp
            else:
                et = show(c.exp) if not isinstance(c.exp, str) else c.exp
            ft = show(c.found) if not isinstance(c.found, str) else c.found
            pend = (not isinstance(c.exp, str) and has_var(c.exp)) or (not isinstance(c.found, str) and has_var(c.found))
            if pend:
                self.notes.append(("omitido-por-pendiente", c.prim, c.code))
                continue
            rel = c.rel
            if c.relkind == "origin":
                dv = []
                rel = src_of_type(c.exp, dv) if not isinstance(c.exp, str) else None
                if dv and rel is not None:
                    self.nonfixed.add("T2 origen de un esperado compuesto sin origen propio (se toma el del componente que lo fijo)")
            if rel is not None and tuple(rel) == tuple(c.prim):
                rel = None  # 4.1.2 N(i)/cl.6: nunca el mismo operando denunciado
            if c.relkind == "none":
                rel = None
            key = (c.prim, c.code, c.sub)
            if (c.prim,) in seen_sites:
                continue
            seen_sites.add((c.prim,))
            d = dict(code=c.code, subcause=c.sub, primary=list(c.prim), expected_type=et, found_type=ft,
                     rel_expected=list(rel) if rel else None, rel_found=list(c.prim) if c.sub != "none" else None,
                     cls=c.cls, clause={"NX-E1303": "cl.4 / 4.7", "NX-E0102": "cl.2-4 / cl.6"}.get(c.code, ""))
            out.append(d)
        main = [d for d in out if d["cls"] == "C"]
        extra = [d for d in out if d["cls"] != "C"]
        # 13.9: orden por start, end, code, subcause (null = "")
        main.sort(key=lambda d: (d["primary"][0], d["primary"][1], d["code"], d["subcause"] or ""))
        return main, extra


def derive(src):
    """Devuelve dict con diagnosticos esperados y notas."""
    try:
        fns = Parser(src).program()
    except Syntax as ex:
        return {"error": "sintaxis fuera del subconjunto: %s" % ex, "diagnostics": [], "notes": [], "unmodeled": ["parser"]}
    T._n = 0
    ck = Checker(src)
    try:
        diags, extra = ck.run(fns)
    except Syntax as ex:
        return {"error": "fuera del subconjunto: %s" % ex, "diagnostics": [], "notes": [], "unmodeled": [str(ex)]}
    return {"diagnostics": diags, "d_optional": extra, "notes": [list(map(str, n)) for n in ck.notes],
            "no_fijado": sorted(ck.nonfixed), "unmodeled": sorted(set(ck.unmodeled))}


def fmt(d):
    """Formato de comparacion de la matriz (columna esperada de `docs/correcciones/I07c-S5c-diseno-ancla.md`)."""
    sc = d["subcause"]
    ab = lambda s: (s or "").replace("Array<", "A<")
    rel = ("exp=[%d,%d)" % tuple(d["rel_expected"])) if d["rel_expected"] else ""
    return "%s%s [%d,%d) %s|%s %s" % (d["code"][3:], "/" + sc[:4] if sc else "", d["primary"][0], d["primary"][1],
                                      ab(d["expected_type"]), ab(d["found_type"]), rel)


# ----------------------------------------------------------------------------------------------------------------
# --self-test: ejemplos literales del ADR (esperados transcritos, no generados)
# ----------------------------------------------------------------------------------------------------------------
ADR = [
    # (fuente, esperado como lista de (code, sub, [s,e), expected_type, found_type, rel_expected), cita)
    ("fn Main(){var r=[];r=[(r==r)==true];}", [], "4.1.2 cl.2 (valido)"),
    ("fn Main(){var r=[];r=[(r==r)==true];r=[1];}",
     [("NX-E0102", "type_mismatch", (39, 40), "Bool", "I64", (21, 35))], "4.1.2 cl.2, precision l.309-324"),
    ("fn Main(){var r=[];r=[r];}", [("NX-E0103", None, (16, 18), "", "", None)], "4.1.2 cl.2, recursivo"),
    ("fn Main(){var r=[true];r=[1];}",
     [("NX-E0102", "type_mismatch", (26, 27), "Bool", "I64", (16, 22))], "4.1.2 gemelo con tipo adelantado"),
    ("fn Main(){var r=[true];r=[true];}", [], "4.1.2 gemelo positivo"),
    ("fn Main(){var r=[];r=[r==r];r=[true];}", [], "4.1.2 ir1c"),
    ("fn Main(){var r=[];var w=[];r=[w==w];w=[true];}", [], "4.1.2 ir1d"),
    ("fn Main(){let q=(if true then (if 1 then 1 else 2) else true);}",
     [("NX-E0102", "type_mismatch", (34, 35), "Bool", "I64", None)], "L T5: condición fallida"),
    ("fn Main(){let p=true;let a=(if p then 1 else [1,true]);}",
     [("NX-E0102", "type_mismatch", (48, 52), "I64", "Bool", (46, 47))], "L N: literal erróneo"),
    ("fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}",
     [("NX-E1303", None, (77, 78), "Array<Array<I64>>", "Array<Array<Array<I64>>>", (53, 57)),
      ("NX-E0102", "type_mismatch", (92, 95), "Array<Array<Array<I64>>>", "Array<Array<I64>>", (84, 87))],
     "L D-REF (invalido)"),
    ("fn Main(){let aar=[[1]];var r=[];let a=(if false then r[0] else aar);r=[[[3]]];}", [], "L D-REF (valido)"),
    ("fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else r));r=[[3]];}", [],
     "L ancla pendiente (valido)"),
    ("fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}",
     [("NX-E1303", None, (83, 86), "Array<Array<I64>>", "Array<Array<Array<I64>>>", (53, 54)),
      ("NX-E0102", "type_mismatch", (92, 93), "Array<I64>", "I64", (74, 77))], "L ancla pendiente (invalido)"),
    ("fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[[3]];}",
     [("NX-E1303", None, (83, 86), "Array<Array<I64>>", "Array<Array<Array<I64>>>", (53, 54))],
     "L control r=[[3]]"),
    ("fn Main(){var q=[];let a=(1 + (if false then q[0] else (if true then 2 else q[0])));q=[3];}", [],
     "L +/- (valido)"),
    ("fn Main(){let ar=[1];var q=[];let a=(ar + (if true then q[0] else (if true then [2] else q)));q=[3];}",
     [("NX-E0102", "type_mismatch", (37, 39), "I64", "Array<I64>", None)], "L +/- (consumo)"),
    ("fn Main(){let ar=[1];var q=[];let a=(1 + (if true then q[0] else (if true then [2] else q)));q=[3];}",
     [("NX-E0102", "type_mismatch", (79, 82), "I64", "Array<I64>", None),
      ("NX-E0102", "type_mismatch", (88, 89), "I64", "Array<I64>", None)], "L +/- (sin consumo)"),
]


def self_test():
    bad = 0
    for src, exp, cite in ADR:
        r = derive(src)
        got = [(d["code"], d["subcause"], tuple(d["primary"]), d["expected_type"], d["found_type"],
                tuple(d["rel_expected"]) if d["rel_expected"] else None) for d in r["diagnostics"]]
        ok = got == [tuple(x) if not isinstance(x, tuple) else x for x in exp]
        if r.get("error") or not ok:
            bad += 1
        print("%-4s %s%s" % ("OK" if ok and not r.get("error") else "FAIL", cite, "" if ok else "\n   got=%s\n   exp=%s" % (got, exp)))
    print("self-test: %d/%d" % (len(ADR) - bad, len(ADR)))
    return bad == 0


# ----------------------------------------------------------------------------------------------------------------
# --matrix
# ----------------------------------------------------------------------------------------------------------------
ROOT = str(Path(__file__).resolve().parents[1])


def load_sources():
    S = {}
    rec = json.load(open(os.path.join(ROOT, "docs/validation/i07c-s5c/fuentes-recuperadas.json"), encoding="utf-8"))
    S.update(rec)
    base = os.path.join(ROOT, "docs/validation/i07c-s5a")
    for n in ["R1", "R1twin", "R2", "R2b", "R3", "R3b", "R3c", "R4", "R5"]:
        S[n] = open(os.path.join(base, "d-repros", n + ".n"), encoding="utf-8").read()
    for n in ["P1", "P2", "P3", "P4", "P4pos", "c3", "w1", "w3", "generated1411", "generated1681"]:
        S[n] = open(os.path.join(base, "limites", n + ".n"), encoding="utf-8").read()
    for c in json.load(open(os.path.join(ROOT, "docs/validation/i07c-twin/casos.json"), encoding="utf-8")):
        if c["id"] in ("O01", "O03", "O07", "O08"):
            S[c["id"]] = c["reduced"]
    return S


ROWS = ["R1", "R1twin", "R3c", "R2", "R2b", "R3", "R3b", "R4", "X_R1na", "X_R1ord", "X_flat3", "Y_pend_if",
        "Y_pend_if2", "X_mix3", "X_R4asg_r", "X_R1inc", "X_R1pre", "X_R2na", "X_R4na", "X_nest_first", "Y_conc_if",
        "Y_pend_if_asg", "W0nounion", "W0", "A", "A_reord", "A_ctrl", "A_noasg", "B_qr", "B_rq", "F06", "M2", "O01",
        "O03", "O07", "O08", "M2_noasg", "M2_ctrl", "DREF_valid", "P1", "c3", "w1", "P2", "P3", "w3"]
NO_DERIVABLE = ["R5", "P4", "P4pos", "generated1411", "generated1681"]
PF = "fn Main(){let p=true;let ar=[1];"
# filas nuevas del plan I07c-S5c-plan.md 5 (esperados transcritos del plan: primario, relacion y cardinalidad)
PLAN_ROWS = {
    "X_join_local": (PF + "var q=[];let a0=(if p then q else (if p then 1 else (if p then true else 3)));q=[[ar]];}",
                     [("NX-E1303", (77, 78), (59, 60))]),
    "X_compat_then_absorbed": (PF + "var q=[];let a0=(if p then q else (if p then 1 else (if p then q else 3)));q=[[ar]];}",
                               [("NX-E1303", (77, 78), (59, 60))]),
    "X_pend_second_leaf": (PF + "var r=[];let a0=(if p then r else (if p then (if p then r else 1) else true));r=[2];}",
                           [("NX-E1303", (95, 96), (59, 60))]),
    "X_two_unions": (PF + "var q=[];var r=[];let a0=(if p then q else (if p then 1 else 2));"
                     "let a1=(if p then r else (if p then true else 3));q=[[ar]];r=[[ar]];}",
                     [("NX-E1303", (86, 87), (68, 69)), ("NX-E1303", (133, 137), (115, 116))]),
    "obs_8bis": (PF + "var q=[];let a0=(if p then q else (if p then q else (if p then 1 else 3)));q=[[ar]];}",
                 [("NX-E1303", None, None)]),
}


def matrix(md_path):
    S = load_sources()
    E = json.load(open(os.path.join(ROOT, "docs/validation/i07c-s5c/oraculo/matriz-esperados.json"), encoding="utf-8"))
    lines, ok_n, res = [], 0, {}
    for n in ROWS:
        r = derive(S[n])
        got = [fmt(d) for d in r["diagnostics"]]
        exp = E[n]
        eq = sorted(got) == sorted(exp)
        # orden estricto (13.9): la matriz lista sus esperados ordenados como cadena; el oraculo, por (start, end, code)
        res[n] = dict(match=eq, got=got, exp=exp, unmodeled=r["unmodeled"], error=r.get("error"),
                      no_fijado=r.get("no_fijado", []), d_optional=[fmt(d) for d in r.get("d_optional", [])])
        ok_n += eq
        lines.append("%-14s %s" % (n, "COINCIDE" if eq else "DISCREPA"))
    print(chr(10).join(lines))
    print("coinciden %d/%d" % (ok_n, len(ROWS)))
    for n, (src, exp) in PLAN_ROWS.items():
        r = derive(src)
        pl = [(d["code"], tuple(d["primary"]), tuple(d["rel_expected"]) if d["rel_expected"] else None)
              for d in r["diagnostics"]]
        if n == "obs_8bis":
            ok = len(pl) == 1 and pl[0][1] == (95, 96)
        else:
            ok = pl == [tuple(x) for x in exp]
        res[n] = dict(got=[fmt(d) for d in r["diagnostics"]], plan=exp, match=ok, unmodeled=r["unmodeled"])
    for n in NO_DERIVABLE:
        r = derive(S[n])
        res[n] = dict(got=[fmt(d) for d in r["diagnostics"]], unmodeled=r["unmodeled"], error=r.get("error"))
    if md_path:
        write_md(md_path, S, res)
    out = os.environ.get("ORACULO_JSON")
    if out:
        json.dump(res, open(out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    return res


DISCREPANCIAS = """## Discrepancias (3 filas) y quien tiene razon

Criterio: el ADR manda; no se fuerza ninguna fila. Los tres casos se verificaron contando bytes de la fuente y comprobando
la coherencia interna de los tipos de la propia fila de la matriz.

### W0 (relacion de la E1303: matriz `[60,64)`, oraculo `[59,63)`) - tiene razon el oraculo (error aritmetico de la matriz)

`fn Main(){let ar=[1];var r=[];var q=[];let a=(if true then true else q);...`: el prefijo `fn Main(){` ocupa 10 bytes,
`let ar=[1];` 11, `var r=[];` 9 y `var q=[];` 9, es decir 39; `let a=(` ocupa `[39,46)`, la condicion `true` `[49,53)`, la primera
continuacion `true` `[59,63)` y `q` `[69,70)`. La relacion `expected_type` es el operando de la primera continuacion
(ADR 4.1.2 clausula 6 / l.371: «para una union, el operando de la primera continuacion que la establece»), esto es `[59,63)`.
La matriz copio `[60,64)` de P1/c3/w1, que llevan un parentesis extra delante (`let a=((if ...`). Los binarios
`main` y V6 tambien imprimen `[59,63)`; el resto de la fila (E1303 `[69,70)` `Bool`/`Array<Array<Bool>>`, sin E0103) coincide.
Accion: corregir la columna esperada de W0 a `exp=[59,63)`.

### P2 y w3 (E1303 y E0102 sobre `r=[r]`) - tiene razon el oraculo; la matriz es internamente contradictoria

Fuente P2: `let a=(if p then [r,q] else (if (x<1) then r else [true]));q=[[3]];q=ar;r=[r];`. Derivacion:
1. `[r,q]` (primera continuacion): referencia = `r` (primer elemento con evidencia valida, 4.1.2 cl.4); la igualdad de `q` con ella se
   acumula (`alfa_r = alfa_q`); tipo del literal `Array<Array<alfa>>`. Ancla de la union = `[r,q]` `[75,80)`.
2. Aplanado en orden fuente: hoja `r` (`[101,102)`), luego `[true]` (`[108,114)`).
3. Con `q=[[3]]` (RHS completo fija `alfa_q := Array<I64>`) el tipo de `r` es `Array<Array<I64>>` y el ancla `Array<Array<Array<I64>>>`
   (la columna esperada de la matriz usa esas mismas cifras, `A^3`). La hoja `r` es incompatible con el ancla (`A^2` frente a `A^3`;
   ademas, antes de conocer `alfa`, `Array<alfa>` frente a `Array<Array<alfa>>` exige `alfa = Array<alfa>`: igualdad recursiva sin solucion
   finita, incompatible por 4.1.2 cl.2 y el addendum «no autoriza la igualdad recursiva»). Es la **primera** hoja posterior incompatible:
   la unica E1303 va sobre `r` `[101,102)` (4.1.2 cl.4, 4.7), con `expected_type=Array<Array<Array<I64>>>`, `found_type=Array<Array<I64>>`
   (tipo completo de la expresion senalada, 13.17/cl.6) y relacion `[75,80)`. `[true]` queda absorbida: no es sitio independiente ni D (cl.4).
4. `q=ar`: E0102 `[127,129)` esperado `Array<Array<I64>>` encontrado `Array<I64>`, relacion = RHS de `q=[[3]]` `[119,124)` (coincide con la matriz).
5. `r=[r]`: el elemento hoja `r` `[133,134)` recibe el tipo de elemento de `r`, `Array<I64>`, y su tipo es `Array<Array<I64>>`:
   E0102 `expected_type=Array<I64>`, `found_type=Array<Array<I64>>`; el origen de ese esperado es la igualdad posterior `q=[[3]]`
   (`[119,124)`), no el ancla.

La matriz (y `main`/V6, de cuya salida se copio; comprobado ejecutando `check` solo para este informe) dice E1303 sobre `[true]` `[108,114)` y
E0102 `[133,134)` con `Array<Array<I64>>`/`Array<Array<Array<I64>>>` y relacion `[75,80)`. Contradiccion interna comprobable sin el ADR: la E1303 de
la propia fila fija el ancla `A^3 = Array<tipo de r>`, es decir `tipo(r) = A^2`; la E0102 de la propia fila exige `found_type = A^3` para la
expresion `r`, es decir `tipo(r) = A^3`. Ambas no pueden valer. Ademas, si `r` fuese compatible con el ancla (`A^2 = A^3`) no habria
solucion finita. w3 es la misma estructura (cambia `q=[[x+1]]`). Cardinalidad (3 diagnosticos) y la E0102 de `q=ar` coinciden; difieren el
primario de la E1303 (`r` frente a `[true]`) y los tipos/relacion de la E0102 de `r=[r]`.
Accion: P2/w3 dejan de ser referencia de N para esos dos diagnosticos; se necesita adjudicacion de B (si B conviene con esta derivacion,
`main` y V6 tienen un defecto previo de nucleo: E1303 sobre una hoja distinta de la primera incompatible). No decidido aqui.
"""

NO_MODELADO = """## Lo que el oraculo no modela

- Filas NO DERIVABLES de la matriz (R5, P4, P4pos, generated1411, generated1681): R5 usa `while`; P4/P4pos usan `if[]then[]else` sin
  espacios y `if()then` (el parser del subconjunto las rechaza con error de sintaxis fuera del subconjunto). generated1411/1681 se
  procesan (seccion final) pero **no se validan** contra la matriz (la matriz las declara sin derivacion cerrada).
- Fuera del subconjunto: asignacion indexada `r[i]=e` y anotaciones `Array<T>`; `AssertEq`; `Drop`; bucles; varias funciones con
  llamadas; cierres; `Never` salvo el literal `{return;}`; dominio `copyable_data` de `==` (se supone satisfecho) y dominio de `<` con
  operandos no I64; el cierre de dominio indeciso de 4.1.2.3 salvo lo que usan `Len` y `<`; relaciones condicionales de proyeccion con
  base que se vuelve escalar mas tarde (solo `not_indexable` inmediato). V (derivados) no se emiten; D (hojas adicionales de una misma
  restriccion) se listan en `d_optional` pero no se comparan (presencia D/V es I, 4.1.2 I(a)).
- Cuotas de solver, pilas y `NX-E0003`: fuera de alcance (andamiaje).
"""


def write_md(path, S, res):
    L = ["# I07c-S5c - oraculo independiente frente a la matriz (E, Flujo B)", ""]
    ok = sum(1 for n in ROWS if res[n]["match"])
    nplan = sum(1 for n in PLAN_ROWS if res[n]["match"])
    L += ["Oraculo: `scripts/i07c_s5c_oraculo.py` (derivado solo del ADR; no lee ni importa el verificador). Fuentes: "
          "`docs/validation/i07c-s5c/fuentes-recuperadas.json`, `docs/validation/i07c-s5a/{d-repros,limites}/`, `docs/validation/i07c-twin/casos.json`. "
          "Esperados de la matriz: columna «Esperado» de `docs/correcciones/I07c-S5c-diseno-ancla.md` (copiados de `rec_all.py` a "
          "`docs/validation/i07c-s5c/oraculo/matriz-esperados.json` por `extraer_esperados.py`, sin ejecutar binarios). "
          "Comparacion: cadena exacta por diagnostico (codigo, subcausa, primario por bytes, `expected_type`/`found_type`, relacion `expected_type`), "
          "como multiconjunto (la matriz lista sus diagnosticos por orden de cadena, no por 13.9; el oraculo emite siempre por 13.9: "
          "inicio, fin, codigo, subcausa).", "",
          "**Resultado: %d/%d filas coinciden; %d discrepan (W0, P2, w3).** Filas nuevas del plan 5 y observacion de 8 bis: %d/%d coinciden en primario y relacion." %
          (ok, len(ROWS), len(ROWS) - ok, nplan, len(PLAN_ROWS)), ""]
    L += [DISCREPANCIAS, "", "## Tabla fila a fila (45 filas)", "",
          "Formato de cada diagnostico: `codigo[/subcausa] [inicio,fin) esperado|encontrado exp=[relacion)`; `A<..>` abrevia `Array<..>`.", "",
          "| # | Fila | Esperado de la matriz | Oraculo | Coincide | Notas |", "|---|---|---|---|---|---|"]
    for i, n in enumerate(ROWS, 1):
        r = res[n]
        notes = []
        if r["no_fijado"]:
            notes.append("decision no fijada " + ", ".join(x.split()[0] for x in r["no_fijado"]) + " (ver `orden-no-fijado.md`)")
        if r["d_optional"]:
            notes.append("D opcional: " + "; ".join(r["d_optional"]))
        if not r["match"]:
            notes.append("**DISCREPA**: ver seccion de discrepancias")
        cell = lambda xs: "<br>".join("`%s`" % x for x in xs) if xs else "(sin diagnosticos)"
        L.append("| %d | %s | %s | %s | %s | %s |" % (i, n, cell(sorted(r["exp"])), cell(r["got"]),
                                                      "si" if r["match"] else "**no**", " ".join(notes)))
    L += ["", "## Filas nuevas del plan 5 y observacion de 8 bis", "",
          "Esperado transcrito del plan (primario y relacion; los tipos los deriva el oraculo).", "",
          "| Fila | Esperado del plan | Oraculo | Coincide |", "|---|---|---|---|"]
    for n, (src, exp) in PLAN_ROWS.items():
        r = res[n]
        if n == "obs_8bis":
            e = "1 E1303 en la hoja `1` (`[95,96)`)"
        else:
            e = "; ".join("E1303 [%d,%d) rel [%d,%d)" % (x[1] + x[2]) for x in exp)
        L.append("| %s | %s | %s | %s |" % (n, e, "<br>".join("`%s`" % x for x in r["got"]), "si" if r["match"] else "**no**"))
    L += ["", NO_MODELADO, "", "## Filas fuera de la matriz (salida del oraculo, sin validar)", ""]
    for n in NO_DERIVABLE:
        r = res[n]
        L.append("- %s: %s" % (n, ("error de sintaxis del subconjunto: " + r["error"]) if r.get("error") else
                               "; ".join("`%s`" % x for x in r["got"])))
    L.append("")
    open(path, "w", encoding="utf-8", newline="\n").write("\n".join(L))


def main(argv):
    if "--self-test" in argv:
        sys.exit(0 if self_test() else 1)
    if "--batch" in argv:
        data = json.load(open(argv[argv.index("--batch") + 1], encoding="utf-8"))
        items = data.items() if isinstance(data, dict) else [(d["id"], d["source"]) for d in data]
        print(json.dumps({k: derive(v) for k, v in items}, ensure_ascii=False))
        return
    if "--source" in argv:
        src = argv[argv.index("--source") + 1]
        print(json.dumps(derive(src), indent=1))
        return
    if "--matrix" in argv:
        matrix(argv[argv.index("--md") + 1] if "--md" in argv else None)
        return
    if len(argv) > 1 and not argv[1].startswith("-"):
        print(json.dumps(derive(open(argv[1], encoding="utf-8").read()), indent=1))
        return
    print(__doc__)


if __name__ == "__main__":
    main(sys.argv)
