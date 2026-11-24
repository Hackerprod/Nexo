#!/usr/bin/env python3
"""Disposable, exact core probes before a full I07c corpus scan."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path("C:/projects/Nexo-i07c")
spec = importlib.util.spec_from_file_location("oracle", ROOT / "scripts/i07c_s5c_oraculo.py")
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
sources = oracle.load_sources()

MINIMA = {
    "t2192": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then q else (if p then 1 else [r]));s=q;q=[true];}",
    "t3584": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));r=q;q=[[ar]];}",
    "t3594": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));q=r;r=[[ar]];}",
    "t3596": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));s=q;q=[true];}",
    "t7094": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [r] else (if p then ar else r));r=q;q=[[ar]];}",
    "planned": "fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}",
    "incompatible_prefix": "fn Main(){let x=1;let p=true;var r=[];var q=[];while [[r],q] {}q=[3];let a=(1<(if x then p else q));r=[3];}",
    "anchored_first_branch": "fn Main(){var r=[];var s=[];let q=(if true then{value:s;}else (if true then{value:(if true then{value:r;}else{value:[];});}else{value:s<1;}));r=[[1]];s=[[]];}",
}
EXPECTED = {
    "t2192": [("NX-E0103", 76, 78)],
    "t3584": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
    "t3594": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
    "t3596": [("NX-E0103", 76, 78), ("NX-E1303", 126, 128)],
    "t7094": [("NX-E0103", 85, 87), ("NX-E1303", 126, 128)],
}

def run(exe, src, flags, name):
    with tempfile.TemporaryDirectory(prefix="rd-cheap-") as tmp:
        source = Path(tmp) / "source.n"
        source.write_bytes(src.encode("utf-8"))
        env = os.environ.copy()
        env["NEXO_XV"] = flags
        p = subprocess.run([exe, "check", "--format", "json", "--profile", "f0-0002-b-v2", str(source)],
                           env=env, text=True, capture_output=True, timeout=60)
        try:
            data = json.loads(p.stdout)
            return sorted([(d["code"], d.get("subcause"), d["primary_span"]["start"], d["primary_span"]["end"])
                           for d in data.get("diagnostics", [])])
        except (ValueError, KeyError, TypeError):
            return [(-1, p.returncode, p.stdout[-200:], p.stderr[-200:])]

def main():
    exe, flags = sys.argv[1:3]
    result = {"flags": flags, "probes": {}, "matrix": {}}
    for name, src in MINIMA.items():
        got = run(exe, src, flags, name)
        simple = sorted((d[0], d[2], d[3]) for d in got)
        expected = EXPECTED.get(name)
        if name == "planned":
            passed = any(d[0] == "NX-E1303" and d[2] == 69 and d[3] == 70 for d in got)
        elif name == "incompatible_prefix":
            passed = len(got) == 4 and [d[0] for d in got] == ["NX-E0102"] * 3 + ["NX-E1303"]
        elif name == "anchored_first_branch":
            passed = simple == [("NX-E0102", 136, 137)]
        else:
            passed = simple == expected
        result["probes"][name] = {"ok": passed, "actual": got}
    for name in oracle.ROWS:
        src = sources[name]
        model = oracle.derive(src)
        expected = sorted((d["code"], d.get("subcause"), d["primary"][0], d["primary"][1])
                          for d in model["diagnostics"])
        got = run(exe, src, flags, name)
        result["matrix"][name] = {"ok": got == expected, "actual": got, "expected": expected} if got != expected else {"ok": True}
    result["summary"] = {"probes_ok": sum(v["ok"] for v in result["probes"].values()),
                         "matrix_ok": sum(v["ok"] for v in result["matrix"].values())}
    print(json.dumps(result, ensure_ascii=False))

if __name__ == "__main__":
    main()
