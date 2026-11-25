#!/usr/bin/env python3
"""Disposable batched I07c differential, with durable per-lot results."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import ctypes
from ctypes import wintypes

EXP = Path("C:/projects/Nexo-i07c/docs/validation/i07c-s5c-explor")
BD2 = Path("C:/projects/Nexo-i07c/docs/validation/i07c-s5c/b-d2")
NEW = Path("C:/projects/Nexo-i07c-rd/docs/validation/i07c-rediseno")
OUT = Path("C:/projects/Nexo-i07c-rd-proto/.scratch/rd-scan")
OUT.mkdir(parents=True, exist_ok=True)
THREAD = threading.local()
MAX_WORKING_SET = 6 * 1024**3
SILENCE_LIMIT = 20 * 60
CORPUS_SHA256 = {
    "c1": "e78702d1ee98178fd2a5809ccfc49c35dcbc3b7438b9664072c5c42742d7cfe9",
    "c2": "514c929f76ef4f6fba3343647353bd420d9d931c03cf039051df5584427cd864",
    "b": "83f9e726254d55802e275b2b18ba270f567513703b5fa10a091ff5cbe410af1f",
    "perm": "8997f93905ae1338fab3a1c921dceed8628516ab4ae8e862e24b6e021700cf72",
    "t": "a1c740bd6aafe417e0a0205fc2759fabd63a6d96507551652b96d971bffce2c5",
    "dirigido5": "684d623488bd2e6ec882036a8bce8162177237ea38a72e4e6d57fb53f2dd5e09",
    "s7021993": "611f34e21b71c06ba255a308a3e01f97cd0f3f0c9de7563ddb2f0961962c0f10",
    "s4417061": "0976fb60d2f494ce5b3beca958da149210c416835c27bcb717e13f26c2cd3c48",
    "s9908255": "d41babb25b6e6b55e5d9832f1ddcff2d2ae12cd31ba2b7100173940e524c586e",
    "c1-nueva": "dba4c8fdd45b9651403463c36cd30626f4a31c8951ad43c80a11802a1090332d",
    "c2-nueva": "ff3de38ed02ca9857d6f073406d1f7c01c3b26dbb6fdadf8d94ff769a7db49e7",
    "c1-nueva2": "52b6f21f934f1fb5d60160ca7864c50fd966d2e1ea2911f9d33ccf42c56aeefa",
    "c2-nueva2": "b61adba671c632a772b2fd5b73bfaf9d8f49006167bfd95df3cafa492604e3b7",
}
ctypes.windll.kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
ctypes.windll.kernel32.OpenProcess.restype = wintypes.HANDLE
ctypes.windll.kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
ctypes.windll.psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD]


class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD),
                ("PeakWorkingSetSize", ctypes.c_size_t), ("WorkingSetSize", ctypes.c_size_t),
                ("QuotaPeakPagedPoolUsage", ctypes.c_size_t), ("QuotaPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t), ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                ("PagefileUsage", ctypes.c_size_t), ("PeakPagefileUsage", ctypes.c_size_t)]


def working_set(pid):
    handle = ctypes.windll.kernel32.OpenProcess(0x1000 | 0x0400, False, pid)
    if not handle:
        return 0
    try:
        counters = PROCESS_MEMORY_COUNTERS()
        counters.cb = ctypes.sizeof(counters)
        if ctypes.windll.psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
            return counters.WorkingSetSize
        return 0
    finally:
        ctypes.windll.kernel32.CloseHandle(handle)

def data(name):
    name = name.split("@", 1)[0]
    if name == "c1": path = EXP / "corpus-congelado.json"
    elif name == "c2": path = EXP / "corpus2-congelado.json"
    elif name == "b": path = BD2 / "corpus-b.json"
    elif name == "perm": path = BD2 / "perm.json"
    elif name == "t": path = BD2 / "corpus-t.json"
    elif name == "dirigido5": path = EXP / "corpus-dirigido5.json"
    elif name.startswith("s") and name[1:] in ("7021993", "4417061", "9908255"):
        path = NEW / ("b-" + name[1:] + ".json")
    elif name in ("c1-nueva", "c2-nueva", "c1-nueva2", "c2-nueva2"):
        path = NEW / (name + ".json")
    else:
        raise ValueError(name)
    content = path.read_bytes()
    digest = hashlib.sha256(content).hexdigest()
    if digest != CORPUS_SHA256[name]:
        raise ValueError(f"corpus SHA-256 mismatch: {name} {digest} != {CORPUS_SHA256[name]}")
    decoded = json.loads(content)
    return decoded["progs"] if name == "perm" else decoded

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
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stdout, tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stderr:
        proc = subprocess.Popen([exe, "check", "--format", "json", "--profile", prof, str(path)],
                                stdout=stdout, stderr=stderr, env=env)
        last_output = time.monotonic()
        last_size = 0
        reason = None
        while proc.poll() is None:
            size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
            if size != last_size:
                last_output, last_size = time.monotonic(), size
            if working_set(proc.pid) > MAX_WORKING_SET:
                reason = "working_set_over_6gb"
                break
            if time.monotonic() - last_output > SILENCE_LIMIT:
                reason = "silent_over_20min"
                break
            time.sleep(0.02)
        if reason:
            proc.kill()  # This PID was created by this run_one invocation.
            proc.wait()
            return {"inconclusive": reason, "pid": proc.pid}
        stdout.seek(0)
        stderr.seek(0)
        output, error = stdout.read(), stderr.read()
    try:
        result = json.loads(output)
        return [signature(d) for d in result.get("diagnostics", [])]
    except (KeyError, ValueError, TypeError):
        return {"error": proc.returncode, "out": output[-300:], "err": error[-300:]}

def run(args):
    rows = data(args.corpus)
    end = min(args.end, len(rows))
    path = OUT / args.tag
    path.mkdir(exist_ok=True)
    with ThreadPoolExecutor(max_workers=4) as pool:
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
    evidence = hashlib.sha256()
    evidence.update(Path(__file__).with_name("i07c_rd_oracle.py").read_bytes())
    if name in ("c2", "c2-nueva"):
        evidence.update((EXP / "analyze2d.py").read_bytes())
    evidence.update(json.dumps(rows, ensure_ascii=False, sort_keys=True).encode("utf-8"))
    identity = evidence.hexdigest()
    if cache.exists():
        cached = json.loads(cache.read_text(encoding="utf-8"))
        if isinstance(cached, dict) and cached.get("identity") == identity:
            return cached["results"]
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
    cache.write_text(json.dumps({"identity": identity, "results": result}, ensure_ascii=False), encoding="utf-8")
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
