import itertools, random
from pathlib import Path
out = Path("tg3"); out.mkdir(exist_ok=True)
PRE = "fn Id(x:view I64)->I64{return x;}"
atoms = ["r", "[r]", "[]", "[[]]", "(1+true)", "[1+true]", "ar", "[ar]", "ar[true]", "[ar[true]]", "1", "true", "[1]", "[true]", "Id(true)", "[[r]]"]
binops = ["==", "<", "+", "-"]
rng = random.Random(20261123)
exprs = set()
for a in atoms:
    exprs.add(a)
for a, b in itertools.product(atoms, atoms):
    for op in binops:
        exprs.add(f"({a}{op}{b})")
    exprs.add(f"[{a},{b}]")
    exprs.add(f"({a} or {b})")
exprs = sorted(exprs)
rng.shuffle(exprs)
exprs = exprs[:450]
ctx = ["let z=@;", "r=@;", "let z=(@==@);", "let z=[@];", "AssertEq(@,@);", "let z=if true then @ else @;"]
n = 0
for e in exprs:
    c = ctx[n % len(ctx)]
    body = c.replace("@", e)
    (out / f"v_{n:04d}.n").write_text(PRE + "fn Main(){ var r=[]; let ar=[1]; " + body + " r=[true]; }", encoding="utf-8")
    n += 1
# closures / names (inner_context) and mixed statements
for k, e in enumerate(exprs[:150]):
    (out / f"v_{n:04d}.n").write_text(PRE + "fn Main(){ var r=[]; let ar=[1]; let f=fn (){ let z=" + e + "; }; f(); r=[[1]]; }", encoding="utf-8"); n += 1
print(n)
