"""Barrido de la familia «union anidada con componente pendiente y asignacion posterior» (S5a.1-r1).

Programas `fn Main(){...var r=[];var q=[];let a0=<union de 3 hojas>;[q=<rhs>;]}` con las dos formas de anidar el `if`.
Oraculo propio derivado de ADR 4.1.2.2 y 4.1.2.4 (no del binario): referencia = tipo de la primera hoja; las demas
hojas en orden aplanado se igualan atomicamente a ella (occurs-check, las compatibles se acumulan); una sola NX-E1303 sobre la
primera incompatible, omitida si sus tipos no son serializables (queda una incognita con su NX-E0103); NX-E0103 por cada
`[]` declarado cuyo elemento sigue sin resolver al cierre; asignacion posterior incompatible con q = NX-E0102.
Predice (hoja con E1303 o None, numero de E0103, hay E0102). Los programas donde el oraculo no decide
(`[q,r]` con elementos incompatibles) se excluyen.

Uso: TEMP=... python -B familia-oraculo.py <salida.json> <exe> [<exe> ...]
Cada fila: fuente, (hoja, E0103, E0102) previstos y, por binario, (hoja con E1303 | None, nº E1303, nº E0103, hay E0102).
"""
import itertools, json, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

PRE = "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];"
class V:
    def __init__(s): s.ref = None
def find(t):
    while isinstance(t, V) and t.ref is not None: t = t.ref
    return t
def occurs(v, t):
    t = find(t)
    return t is v or (isinstance(t, tuple) and occurs(v, t[1]))
def unify(a, b, trail):
    a, b = find(a), find(b)
    if a is b: return True
    if isinstance(a, V):
        if occurs(a, b): return False
        a.ref = b; trail.append(a); return True
    if isinstance(b, V): return unify(b, a, trail)
    if a == "I" or b == "I": return a == b
    return unify(a[1], b[1], trail)
def atomic(a, b):
    trail = []
    if unify(a, b, trail): return True
    for v in trail: v.ref = None
    return False
def has_var(t):
    t = find(t)
    return isinstance(t, V) or (isinstance(t, tuple) and has_var(t[1]))
A = lambda t: ("A", t)
RHS = {"[[ar]]": A(A(A("I"))), "[ar]": A(A("I")), "[aar]": A(A(A("I"))), "[3]": A("I"), "[[[1]]]": A(A(A(A("I"))))}
def predict(leaves, rhs):
    q, r = V(), V()
    env = {"q": A(q), "r": A(r), "1": "I", "2": "I", "ar": A("I"), "aar": A(A("I")), "r3": A(A(A("I")))}
    def ty(l):
        if l == "[q,r]": return A(env["q"]) if atomic(env["q"], env["r"]) else None
        return env[l]
    ref = ty(leaves[0])
    if ref is None: return None
    first = None
    for i, l in enumerate(leaves[1:], 1):
        t = ty(l)
        if t is None: return None
        if not atomic(ref, t) and first is None: first = (i, ref, t)
    e0102 = False
    if rhs is not None and not atomic(env["q"], RHS[rhs]): e0102 = True
    e0103 = sum(has_var(env[n][1]) for n in "qr")
    leaf = None
    if first and not (has_var(first[1]) or has_var(first[2])): leaf = first[0]
    return leaf, e0103, e0102

def programs():
    leaves = ["q", "r", "1", "2", "ar", "aar", "r3", "[q,r]"]
    for shape in "RL":
        for ls in itertools.product(leaves, repeat=3):
            if "q" not in ls and "[q,r]" not in ls: continue
            for rhs in [None, "[[ar]]", "[ar]", "[aar]", "[3]", "[[[1]]]"]:
                u = (f"(if p then {ls[0]} else (if p then {ls[1]} else {ls[2]}))" if shape == "R"
                     else f"(if p then (if p then {ls[0]} else {ls[1]}) else {ls[2]})")
                want = predict(list(ls), rhs)
                if want is None: continue
                yield PRE + "var r=[];var q=[];" + f"let a0={u};" + (f"q={rhs};" if rhs else "") + "}", ls, want
def leaf_starts(src, ls):
    cur, out = src.index("let a0="), []
    for l in ls:
        cur = src.index(l, cur); out.append(cur); cur += len(l)
    return out
def run(exe, src, d, ls):
    p = os.path.join(d, "c.n"); open(p, "wb").write(src.encode())
    out = subprocess.run([exe, "check", p, "--profile", "f0-0002-b-v2", "--format", "json"], capture_output=True)
    ds = json.loads(out.stdout)["diagnostics"]
    e13 = [g for g in ds if g["code"] == "NX-E1303"]
    starts = leaf_starts(src, ls)
    leaf = starts.index(e13[0]["primary_span"]["start"]) if e13 and e13[0]["primary_span"]["start"] in starts else (-1 if e13 else None)
    return [leaf, len(e13), sum(g["code"] == "NX-E0103" for g in ds), any(g["code"] == "NX-E0102" for g in ds)]
def main():
    out, exes = sys.argv[1], sys.argv[2:]
    progs = list(programs())
    def work(it):
        src, ls, want = it
        with tempfile.TemporaryDirectory(prefix="fam-", dir=os.environ["TEMP"]) as d:
            return [run(e, src, d, ls) for e in exes]
    with ThreadPoolExecutor(4) as pool: res = list(pool.map(work, progs))
    json.dump([[s, list(w[:1]) + [w[1], w[2]], r, list(ls)] for (s, ls, w), r in zip(progs, res)], open(out, "w"))
    print(len(progs), "programas")
if __name__ == "__main__": main()
