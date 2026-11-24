"""tg4: deep arrays inside guarded match arms / if / while bodies; tg5: index selectors over unresolved or union bases.
Usage: python -B gen_tg45.py <out_root>"""
import sys
from pathlib import Path
root = Path(sys.argv[1])
def arr(inner, d): return "[" * d + inner + "]" * d
o4 = root / "tg4"; o4.mkdir(parents=True, exist_ok=True); n = 0
for d in (124, 125, 126, 127, 128, 129, 130, 140):
    for body in ("let x:Bool=%s;", "var r=%s; r=[true];", "AssertEq(%s,true);", "let x=%s==true;"):
        b = body % arr("1", d)
        for wrap in ("match 1{1 if true=>{B},_=>{}}", "match 1{_=>{B}}", "match 1{1 if true=>{},_=>{B}}", "match 1{1 if true=>{B},2 if true=>{B},_=>{}}", "if true {B}", "while false {B}"):
            (o4 / f"w_{n:04d}.n").write_text("fn Main(){" + wrap.replace("B", b) + "}", encoding="utf-8"); n += 1
o5 = root / "tg5"; o5.mkdir(parents=True, exist_ok=True); n = 0
bases = ["r", "[r]", "(if true then r else [1])", "(r or [1])", "(if true then [1] else None)", "ar", "(1+true)", "u", "o", "(r)"]
pre = "var r=[]; let ar=[1]; let u=if true then [1] else [true]; let o:Option<I64>=None; "
for b in bases:
    for sel in ("0", "[1,true]", arr("1", 126), arr("1", 128), arr("1", 129), arr("1", 131), arr("true", 129)):
        for tail in ("", "r=[true];", "r=[[1]];"):
            (o5 / f"x_{n:04d}.n").write_text("fn Main(){" + pre + "let z=" + b + "[" + sel + "]; " + tail + "}", encoding="utf-8"); n += 1
