"""Differential runner for I07c-S6-COR-2: base vs candidate, debug and release, check and run.

Usage: python -B diff_run.py <corpus_dir> <out.jsonl> <start> <count> <workers> <subset> base_exe cand_exe

<subset> is `all` or `debug`: the debug subset (the cost of a debug run that reaches the 1.000.000
tick quota is ~4 s) is every program without `while` plus every 8th program of the sorted corpus.

For every source (sorted by name, slice [start, start+count)) and every profile in PROFILES and
verb in VERBS it runs the four binaries from inside the corpus directory (so the file name in the
JSON diagnostics is identical) and appends one JSON line per source with
{"name", "sha", "cells": {"<profile>|<verb>": [[exit, sha(stdout)] x 2]}, "texts": {...}}; `texts`
keeps the stdouts only for the cells where base and candidate disagree, so the result file stays small.
"""

import hashlib
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

PROFILES = ["f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1"]
VERBS = ["check", "run"]


def invoke(exe: str, verb: str, name: str, profile: str, cwd: str):
    try:
        done = subprocess.run(
            [exe, verb, name, "--profile", profile, "--format", "json"],
            cwd=cwd, capture_output=True, timeout=120,
        )
    except subprocess.TimeoutExpired:
        return [-999, "timeout", ""]
    out = done.stdout
    return [done.returncode, hashlib.sha256(out).hexdigest()[:16], out.decode("utf-8", "replace") if len(out) < 20000 else out[:20000].decode("utf-8", "replace")]


def one(args):
    corpus, name, exes = args
    path = Path(corpus) / name
    cells, texts = {}, {}
    for profile in PROFILES:
        for verb in VERBS:
            runs = [invoke(exe, verb, name, profile, corpus) for exe in exes]
            key = f"{profile}|{verb}"
            cells[key] = [[r[0], r[1]] for r in runs]
            digests = {(r[0], r[1]) for r in runs}
            if len(digests) > 1:
                texts[key] = [r[2] for r in runs]
    return {"name": name, "sha": hashlib.sha256(path.read_bytes()).hexdigest(), "cells": cells, "texts": texts}


def main() -> None:
    corpus, out, start, count, workers = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
    subset, exes = sys.argv[6], sys.argv[7:9]
    names = sorted(p.name for p in Path(corpus).glob("*.n"))
    if subset == "debug":
        names = [n for i, n in enumerate(names)
                 if i % 8 == 0 or b"while" not in (Path(corpus) / n).read_bytes()]
    names = names[start:start + count]
    done = set()
    if Path(out).exists():
        done = {json.loads(line)["name"] for line in Path(out).read_text(encoding="utf-8").splitlines() if line}
    todo = [(corpus, n, exes) for n in names if n not in done]
    with ThreadPoolExecutor(workers) as pool, open(out, "a", encoding="utf-8", newline="\n") as handle:
        for row in pool.map(one, todo):
            handle.write(json.dumps(row) + "\n")
            handle.flush()
    print(f"ran {len(todo)} (skipped {len(done & set(names))}) of slice {start}+{count}")


if __name__ == "__main__":
    main()
