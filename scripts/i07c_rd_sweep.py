#!/usr/bin/env python3
"""Disposable batched I07c differential, with durable per-lot results."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading

EXP = Path("C:/projects/Nexo-i07c/docs/validation/i07c-s5c-explor")
BD2 = Path("C:/projects/Nexo-i07c/docs/validation/i07c-s5c/b-d2")
NEW = Path("C:/projects/Nexo-i07c-rd/docs/validation/i07c-rediseno")
OUT = Path("C:/projects/Nexo-i07c-rd-proto/.scratch/rd-scan")
OUT.mkdir(parents=True, exist_ok=True)
THREAD = threading.local()

def data(name):
    name = name.split("@", 1)[0]
    if name == "c1": return json.loads((EXP / "corpus-congelado.json").read_text(encoding="utf-8"))
    if name == "c2": return json.loads((EXP / "corpus2-congelado.json").read_text(encoding="utf-8"))
    if name == "b": return json.loads((BD2 / "corpus-b.json").read_text(encoding="utf-8"))
    if name == "perm": return json.loads((BD2 / "perm.json").read_text(encoding="utf-8"))["progs"]
    if name == "t": return json.loads((BD2 / "corpus-t.json").read_text(encoding="utf-8"))
    if name == "dirigido5": return json.loads((EXP / "corpus-dirigido5.json").read_text(encoding="utf-8"))
    if name.startswith("s") and name[1:] in ("7021993", "4417061", "9908255"):
        return json.loads((NEW / ("b-" + name[1:] + ".json")).read_text(encoding="utf-8"))
    if name in ("c1-nueva", "c2-nueva"):
        return json.loads((NEW / (name + ".json")).read_text(encoding="utf-8"))
    raise ValueError(name)

def source(row):
    return row if isinstance(row, str) else row.get("wrapped", row.get("w"))

def profile(name):
    if "@" in name: return name.split("@", 1)[1]
    return "f0-0003-b-v1" if name.startswith("s") and name[1:].isdigit() else "f0-0002-b-v2"

def signature(diag):
    rel = next(((r["span"]["start"], r["span"]["end"]) for r in diag.get("related_spans", [])
                if r["role"] == "expected_type"), None)
    d = diag.get("data") or {}
    return [diag["code"], diag.get("subcause"), diag["primary_span"]["start"], diag["primary_span"]["end"],
            d.get("expected_type"), d.get("found_type"), rel]

def run_one(args):
    exe, flags, prof, row = args
    if not hasattr(THREAD, "dir"):
        THREAD.dir = tempfile.TemporaryDirectory(prefix="i07c-rd-batch-")
    path = Path(THREAD.dir.name) / "source.n"
    path.write_bytes(source(row).encode("utf-8"))
    env = os.environ.copy()
    if flags is not None: env["NEXO_XV"] = flags
    proc = subprocess.run([exe, "check", "--format", "json", "--profile", prof, str(path)],
                          capture_output=True, text=True, timeout=120, env=env)
    try:
        result = json.loads(proc.stdout)
        return [signature(d) for d in result.get("diagnostics", [])]
    except (KeyError, ValueError, TypeError):
        return {"error": proc.returncode, "out": proc.stdout[-300:], "err": proc.stderr[-300:]}

def run(args):
    rows = data(args.corpus)
    end = min(args.end, len(rows))
    path = OUT / args.tag
    path.mkdir(exist_ok=True)
    with ThreadPoolExecutor(max_workers=6) as pool:
        results = list(pool.map(run_one, ((args.exe, args.flags, profile(args.corpus), r)
                                          for r in rows[args.start:end])))
    dest = path / f"{args.corpus}-{args.start:05d}-{end:05d}.json"
    dest.write_text(json.dumps({"tag": args.tag, "corpus": args.corpus, "start": args.start,
                                "end": end, "flags": args.flags, "results": results}, ensure_ascii=False),
                    encoding="utf-8")
    print(json.dumps({"path": str(dest), "n": len(results), "errors": sum(isinstance(r, dict) for r in results)}))

def load(tag, name):
    path = OUT / tag
    files = sorted(path.glob(name + "-*.json"))
    if not files: raise FileNotFoundError(str(path / (name + "-*.json")))
    values = {}
    for f in files:
        doc = json.loads(f.read_text(encoding="utf-8"))
        for i, v in enumerate(doc["results"], doc["start"]):
            if i in values and values[i] != v: raise ValueError((tag, name, i, "overlap"))
            values[i] = v
    return values

def oracle_module():
    spec = importlib.util.spec_from_file_location("i07c_s5c_oraculo", Path(__file__).with_name("i07c_rd_oracle.py"))
    module = importlib.util.module_from_spec(spec)
    sys.modules["i07c_s5c_oraculo"] = module
    spec.loader.exec_module(module)
    return module

def oracle_sig(result):
    if result is None or result.get("unmodeled") or result.get("error"): return None
    return [[d["code"], d.get("subcause"), *d["primary"], d.get("expected_type"),
             d.get("found_type"), d.get("rel_expected")] for d in result["diagnostics"]]

def oracle_wrapped(model, row):
    if row["group"] != "W":
        sys.path.insert(0, str(EXP))
        sys.path.insert(0, str(EXP.parents[2] / "scripts"))
        import analyze2d
        try:
            analog = analyze2d.analog(row)
            return [list(d) + [None] if len(d) == 6 else list(d) for d in analog] if analog is not None else None
        except Exception:
            return None
    try:
        original = oracle_sig(model.derive(row["flat"]))
    except Exception:
        return None
    if original is None: return None
    def mapped(pos):
        for first, end, wrapped in row["segs"]:
            if first <= pos < end or pos == end and first < end:
                return pos - first + wrapped
        return None
    result = []
    for diag in original:
        a, b = mapped(diag[2]), mapped(diag[3])
        rel = [mapped(diag[6][0]), mapped(diag[6][1])] if diag[6] else None
        if a is None or b is None or rel is not None and None in rel: return None
        result.append([diag[0], diag[1], a, b, diag[4], diag[5], rel])
    return result

def expected(name, rows):
    name = name.split("@", 1)[0]
    cache = OUT / ("oracle-" + name + ".json")
    if cache.exists(): return json.loads(cache.read_text(encoding="utf-8"))
    if name in ("t", "dirigido5"):
        result = [None] * len(rows)
    else:
        model = oracle_module()
        if name in ("c2", "c2-nueva"):
            result = [oracle_wrapped(model, r) for r in rows]
        else:
            result = []
            for r in rows:
                try:
                    result.append(oracle_sig(model.derive(r["flat"] if isinstance(r, dict) and r.get("form") in ("flat", "blockstyle")
                                                          else source(r))))
                except Exception:
                    result.append(None)
    cache.write_text(json.dumps(result, ensure_ascii=False), encoding="utf-8")
    return result

def table(args):
    rows = data(args.corpus)
    model = expected(args.corpus, rows)
    base, cand = load(args.base, args.corpus), load(args.candidate, args.corpus)
    keys = sorted(set(base) & set(cand))
    if not keys: raise ValueError("no overlapping lots")
    result = {"corpus": args.corpus, "n": len(keys), "unmodeled": 0,
              "core": {k: 0 for k in "ABCD"}, "full": {k: 0 for k in "ABCD"},
              "ok": {"core": 0, "full": 0}, "same_unmodeled": 0,
              "changed_unmodeled": 0, "regressions": []}
    for i in keys:
        b, c, o = base[i], cand[i], model[i]
        if isinstance(b, dict) or isinstance(c, dict): raise ValueError((i, b, c))
        if o is None:
            result["unmodeled"] += 1
            result["same_unmodeled" if b == c else "changed_unmodeled"] += 1
            continue
        for level in ("core", "full"):
            short_full = args.corpus.split("@", 1)[0] in ("c2", "c2-nueva") and isinstance(rows[i], dict) and rows[i].get("form") in (
                "annot", "arg", "ret", "option", "result", "takeempty", "matchexpr")
            select = (lambda x: [d[:4] for d in x]) if level == "core" else (
                (lambda x: [d[:6] for d in x]) if short_full else (lambda x: x))
            bb, cc, oo = select(b), select(c), select(o)
            category = "A" if bb == oo and cc != oo else "B" if bb != oo and cc == oo else "OK" if bb == oo else "C" if bb == cc else "D"
            if category == "OK":
                result["ok"][level] += 1
            else:
                result[level][category] += 1
                if category == "A" and level == "core": result["regressions"].append(i)
    print(json.dumps(result, ensure_ascii=False))

def main():
    p = argparse.ArgumentParser()
    sp = p.add_subparsers(dest="command", required=True)
    runp = sp.add_parser("run")
    runp.add_argument("tag")
    runp.add_argument("corpus")
    runp.add_argument("exe")
    runp.add_argument("--flags")
    runp.add_argument("--start", type=int, default=0)
    runp.add_argument("--end", type=int, default=1000)
    tab = sp.add_parser("table")
    tab.add_argument("base")
    tab.add_argument("candidate")
    tab.add_argument("corpus")
    args = p.parse_args()
    {"run": run, "table": table}[args.command](args)

if __name__ == "__main__": main()
