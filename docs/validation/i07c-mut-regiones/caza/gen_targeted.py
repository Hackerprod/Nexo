"""Targeted sources for the survivors (free-form templates around the mutated construct)."""
import itertools, sys
from pathlib import Path
out = Path(sys.argv[1]); out.mkdir(exist_ok=True)
PRE = "fn Id(x:view I64)->I64{return x;}"
bad = ["(1+true)", "(true+true)", "undefined_name", "ar[true]", "(Id)(true)", "Id(true)"]
ctx = [
 "let z=@;", "let z=@+1;", "let z=[@];", "let z=(true+@);", "let z=if true then @ else 2;",
 "var r=[]; let z=(true+([r] or r)); r=[true];", "var r=[]; let z=@; r=[[1]]; r=[true];",
 "var r=[]; r=[@]; r=[true];", "let ar=[1]; let z=ar[@];", "let ar=[1]; let z=@[0];", "var s=[]; let z=@; s=[[3]];",
 "let z=[@, 1];", "AssertEq(@,1);", "@;", "let f=@; f(1);", "var r=[]; let q=[r]; let z=@(q);",
]
callee = ["(1+true)(2)", "(Id)(true)(1)", "ar2(1)", "(1)(2)", "(Id(true))(3)", "((1+true))(2)", "[1](2)"]
n = 0
for c, b in itertools.product(ctx, bad + callee):
    body = c.replace("@", b)
    for wrap in ("{B}", "{ {} B }", "while false { } B", "if false { } B"):
        src = PRE + "fn Main(){ " + wrap.replace("B", body) + " }"
        (out / f"t_{n:04d}.n").write_text(src, encoding="utf-8"); n += 1
# match-arm / guard negatives for parse.rs
arms = ["1 if true 2 => 3", "1 if true => 3", "Some(1) if 2 3 => 4", "1 if 2 3", "_ if true", "1 if x=>y", "(1) if true 2"]
for a in arms:
    for pre in ("let x=1; ", "let x=Some(1); "):
        (out / f"t_{n:04d}.n").write_text(PRE + "fn Main(){ " + pre + "match x { " + a + " } }", encoding="utf-8"); n += 1
        (out / f"t_{n:04d}.n").write_text(PRE + "fn Main(){ " + pre + "let y = match x { " + a + " }; }", encoding="utf-8"); n += 1
print(n)
