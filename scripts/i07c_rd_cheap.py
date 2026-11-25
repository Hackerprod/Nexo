#!/usr/bin/env python3
"""Disposable, exact core probes before a full I07c corpus scan."""
import json
import sys

import i07c_rd_oracle as oracle
import i07c_rd_sweep as sweep
sources = oracle.load_sources()

MINIMA = {
    "t2192": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then q else (if p then 1 else [r]));s=q;q=[true];}",
    "t3584": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));r=q;q=[[ar]];}",
    "t3584_min": "fn Main(){var q=[];var r=[];(if true then [q] else (if true then [1] else q));r=q;q=[1];}",
    "w0_min": "fn Main(){var r=[];var q=[];(if true then true else q);[r,q];q=[[true]];r=[];}",
    "projection_unrelated": "fn Main(){var s=[];let b=s[0];var q=[];var r=[];(if true then [q] else (if true then [1] else q));r=q;q=[1];s=[1];}",
    "projection_nested_c1_1976": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=[(if p then (if p then r[0] else q) else (if p then [q,r] else r))];q=[ar];r=q;}",
    "projection_union_97": "fn Main(){var q=[];var r=[];let a=(if true then r[0] else (if true then q else r));r=q;r=[true];}",
    "t3594": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));q=r;r=[[ar]];}",
    "t3596": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));s=q;q=[true];}",
    "t7094": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [r] else (if p then ar else r));r=q;q=[[ar]];}",
    "planned": "fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}",
    "planned_full": "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];var q=[];var s=[];var i0:Array<Array<Array<I64>>>=[[[0]]];i0[0]=([s[x],(if 0 then true else q),(if x then q else s)] == x);let a1=s;while Len(q) {}r=br;q=br;s=[[true]];}",
    "incompatible_prefix": "fn Main(){let x=1;let p=true;var r=[];var q=[];while [[r],q] {}q=[3];let a=(1<(if x then p else q));r=[3];}",
    "anchored_first_branch": "fn Main(){var r=[];var s=[];let q=(if true then{value:s;}else (if true then{value:(if true then{value:r;}else{value:[];});}else{value:s<1;}));r=[[1]];s=[[]];}",
    "recursive": "fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}",
    "positive": "fn Main(){let p=true;let ar=[1];var q=[];let a=(if p then q else ar);q=ar;}",
}
EXPECTED = {
    "t2192": [("NX-E0103", 76, 78)],
    "t3584": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
    "t3584_min": [("NX-E1303", 65, 68)],
    "w0_min": [("NX-E1303", 52, 53)],
    "projection_unrelated": [("NX-E1303", 85, 88)],
    "projection_nested_c1_1976": [("NX-E0103", 85, 87), ("NX-E0102", 150, 151),
                                  ("NX-E1303", 158, 159), ("NX-E0102", 172, 173)],
    "projection_union_97": [("NX-E0103", 16, 18)],
    "t3594": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
    "t3596": [("NX-E0103", 76, 78), ("NX-E1303", 126, 128)],
    "t7094": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
}

def run(exe, src, flags, name):
    result = sweep.run_one((exe, flags, "f0-0002-b-v2", src))
    if isinstance(result, dict):
        return [(-1, result)]  # INCONCLUSO; never cached as a passing probe.
    return [(row[0], row[1], row[2], row[3]) for row in result]

def main():
    exe, flags = sys.argv[1:3]
    result = {"flags": flags, "probes": {}, "matrix": {}}
    for name, src in MINIMA.items():
        got = run(exe, src, flags, name)
        if got and got[0][0] == -1:
            result["probes"][name] = {"ok": False, "inconclusive": got[0][1]}
            continue
        simple = [(d[0], d[2], d[3]) for d in got]
        expected = EXPECTED.get(name)
        if name in ("planned", "planned_full"):
            at = src.index("true else q") + len("true else ")
            passed = any(d[0] == "NX-E1303" and d[2] == at and d[3] == at + 1 for d in got)
        elif name == "incompatible_prefix":
            passed = len(got) == 4 and [d[0] for d in got] == ["NX-E0102"] * 3 + ["NX-E1303"]
        elif name == "anchored_first_branch":
            passed = simple == [("NX-E0102", 136, 137)]
        elif name == "recursive":
            passed = ("NX-E0103", 66, 67) in simple
        elif name == "positive":
            passed = not got
        else:
            passed = simple == expected
        result["probes"][name] = {"ok": passed, "actual": got}
    for name in oracle.ROWS:
        src = sources[name]
        model = oracle.derive(src)
        expected = [(d["code"], d.get("subcause"), d["primary"][0], d["primary"][1])
                    for d in model["diagnostics"]]
        got = run(exe, src, flags, name)
        if got and got[0][0] == -1:
            result["matrix"][name] = {"ok": False, "inconclusive": got[0][1]}
            continue
        result["matrix"][name] = {"ok": got == expected, "actual": got, "expected": expected} if got != expected else {"ok": True}
    result["summary"] = {"probes_ok": sum(v["ok"] for v in result["probes"].values()),
                         "matrix_ok": sum(v["ok"] for v in result["matrix"].values())}
    print(json.dumps(result, ensure_ascii=False))

if __name__ == "__main__":
    main()
