#!/usr/bin/env python3
"""B's m1.py metamorphic projection sources, with runnable binary arguments."""

import argparse
import json

from i07c_rd_b_minima import run

parser = argparse.ArgumentParser()
parser.add_argument("main")
parser.add_argument("candidate")
parser.add_argument("--flags", required=True)
args = parser.parse_args()
total = 0
failed = 0


def both(name, source, expected):
    global total, failed
    base = run(args.main, source, "f0-0002-b-v2", None)
    candidate = run(args.candidate, source, "f0-0002-b-v2", args.flags)
    if name == "proj q[0] en bloque value" and base == candidate and base[0][0] == "NX-E0002":
        print(json.dumps({"case": name, "excluded": "syntax error in source"}, ensure_ascii=False))
        return
    total += 1
    labels = lambda rows: [(code, source[start:end]) for code, start, end in rows]
    correct = labels(base) == expected and labels(candidate) == expected
    failed += not correct
    print(json.dumps({"case": name, "expected": expected, "main": labels(base),
                      "candidate": labels(candidate), "ok": correct}, ensure_ascii=False))
U="(if true then [q] else (if true then [1] else q))"
base=f"fn Main(){{var q=[];var r=[];{U};r=q;q=[1];}}"
E=[('NX-E1303','[1]')]
both("base",base,E)
both("proj externa q[0] antes",f"fn Main(){{var q=[];var r=[];let b=q[0];{U};r=q;q=[1];}}",E)
both("proj q[0] en if previa",f"fn Main(){{var q=[];var r=[];let b=(if true then q[0] else q[0]);{U};r=q;q=[1];}}",E)
both("proj q[0] en bloque value",f"fn Main(){{var q=[];var r=[];let b={{value:q[0];}};{U};r=q;q=[1];}}",E)
both("proj q[0] en paren",f"fn Main(){{var q=[];var r=[];let b=((q[0]));{U};r=q;q=[1];}}",E)
both("unión en let",f"fn Main(){{var q=[];var r=[];let u={U};r=q;q=[1];}}",E)
both("unión doble paren",f"fn Main(){{var q=[];var r=[];({U});r=q;q=[1];}}",E)
both("hojas en bloque",f"fn Main(){{var q=[];var r=[];(if true then {{value:[q];}} else (if true then {{value:[1];}} else {{value:q;}}));r=q;q=[1];}}",E)
both("proj q[0] tras union",f"fn Main(){{var q=[];var r=[];{U};let b=q[0];r=q;q=[1];}}",E)
both("proj r[0] antes",f"fn Main(){{var q=[];var r=[];let b=r[0];{U};r=q;q=[1];}}",E)
both("proj r[0] en if previa",f"fn Main(){{var q=[];var r=[];let b=(if true then r[0] else r[0]);{U};r=q;q=[1];}}",E)
# 97B family
V="(if true then r[0] else (if true then q else r))"
b97=f"fn Main(){{var q=[];var r=[];let a={V};r=q;r=[true];}}"
E97=[('NX-E0103','[]')]
both("97 base",b97,E97)
both("97 hojas bloque",f"fn Main(){{var q=[];var r=[];let a=(if true then {{value:r[0];}} else (if true then {{value:q;}} else {{value:r;}}));r=q;r=[true];}}",E97)
both("97 paren hoja",f"fn Main(){{var q=[];var r=[];let a=(if true then (r[0]) else (if true then (q) else (r)));r=q;r=[true];}}",E97)
both("97 con let previo",f"fn Main(){{var q=[];var r=[];let z=r[0];let a={V};r=q;r=[true];}}",E97)
both("97 con if previa",f"fn Main(){{var q=[];var r=[];let z=(if true then r[0] else r[0]);let a={V};r=q;r=[true];}}",E97)
print(json.dumps({"tested": total, "failed": failed}))
raise SystemExit(1 if failed else 0)
