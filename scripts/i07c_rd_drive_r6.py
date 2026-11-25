#!/usr/bin/env python3
"""Resume the R6 disposable sweep, one locked batch at a time."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import i07c_rd_sweep as sweep

BASE = "C:/nexo-work/target-rd-main/release/nexo.exe"
CANDIDATE = "C:/nexo-work/target-r6/release/nexo.exe"
FLAGS = "pendfa,retfa,nojoin,atomic0,e0103rule,retfail,nojoinr,skipv3,r5prefix,r1t5,r5t5,noshape,r6witness"
CORPORA = [
    "b-projections-2026112401", "b-projections-t5-2026112402",
    "b-projections-2026112403", "owner-projections-771412",
    "c1", "c2", "c2@f0-0003-b-v1", "b", "perm", "t",
    "s7021993", "s4417061", "s9908255", "dirigido5",
    "c1-nueva", "c2-nueva", "c2-nueva@f0-0003-b-v1",
    "c1-nueva2", "c2-nueva2", "c2-nueva2@f0-0003-b-v1",
]
DEST = Path("docs/validation/i07c-rediseno")


def free_virtual_memory_kb():
    process = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory"],
        capture_output=True, text=True, check=True,
    )
    return int(process.stdout.strip())


def covered(tag, corpus, count):
    try:
        return len(sweep.load(tag, corpus)) == count
    except FileNotFoundError:
        return False


def locked_run(tag, corpus, executable, flags, start, end):
    inner = [sys.executable, "-B", "scripts/i07c_rd_sweep.py", "run", tag,
             corpus, executable, "--start", str(start), "--end", str(end)]
    locked = os.environ.get("I07C_RD_LOCK_HELD") == "1"
    command = inner if locked else [sys.executable, "-B", "scripts/heavy-lock.py",
                                    "run", "--session", "i07c-rediseno", "--", *inner]
    if flags:
        command.extend(["--flags", flags])
    env = os.environ.copy()
    env["CARGO_BUILD_JOBS"] = "4"
    env["NEXTEST_TEST_THREADS"] = "4"
    memory_waits = 0
    while True:
        if free_virtual_memory_kb() <= 25 * 1024 * 1024:
            print("FREE VIRTUAL MEMORY <= 25 GiB; waiting", flush=True)
            time.sleep(120)
            memory_waits += 1
            if memory_waits >= 10:
                raise RuntimeError("free virtual memory unavailable for 20 min")
            continue
        memory_waits = 0
        result = subprocess.run(command, capture_output=True, text=True, env=env)
        if result.returncode == 0:
            print(result.stdout.strip(), flush=True)
            if "\"errors\": 0" not in result.stdout:
                raise RuntimeError(f"inconclusive batch {tag} {corpus} {start}:{end}")
            return
        if "trabajo pesado ocupado" not in result.stderr + result.stdout:
            raise RuntimeError(f"batch failed: {result.stdout} {result.stderr}")
        print("heavy lock occupied; waiting 60 s", flush=True)
        time.sleep(60)


def main():
    base_hash = hashlib.sha256(Path(BASE).read_bytes()).hexdigest()
    candidate_hash = hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest()
    if not base_hash.startswith("44bef374"):
        raise RuntimeError(f"unexpected main binary {base_hash}")
    base_tag = "main-" + base_hash[:12]
    candidate_tag = "proto-" + candidate_hash[:12] + "-r6-r5"
    session_started = time.monotonic()
    print(json.dumps({"base_sha256": base_hash, "candidate_sha256": candidate_hash,
                      "base_tag": base_tag, "candidate_tag": candidate_tag}), flush=True)
    selection = sys.argv[1:] or CORPORA
    for corpus in selection:
        if corpus not in CORPORA:
            raise ValueError(corpus)
        rows = sweep.data(corpus)
        for tag, executable, flags in ((base_tag, BASE, None),
                                        (candidate_tag, CANDIDATE, FLAGS)):
            if covered(tag, corpus, len(rows)):
                print(f"reuse {tag} {corpus} {len(rows)}", flush=True)
                continue
            try:
                existing = sweep.load(tag, corpus)
            except FileNotFoundError:
                existing = {}
            for start in range(0, len(rows), 2000):
                if os.environ.get("I07C_RD_LOCK_HELD") == "1" and time.monotonic() - session_started > 18 * 60:
                    print("locked batch reached 18 min; resume later", flush=True)
                    return
                end = min(start + 2000, len(rows))
                if all(i in existing for i in range(start, end)):
                    continue
                locked_run(tag, corpus, executable, flags, start, end)
                if os.environ.get("I07C_RD_LOCK_HELD") != "1":
                    time.sleep(65)
        result = subprocess.run(
            [sys.executable, "-B", "scripts/i07c_rd_sweep.py", "table",
             base_tag, candidate_tag, corpus], capture_output=True, text=True, check=True,
        )
        summary = json.loads(result.stdout)
        summary.update({"base_sha256": base_hash, "candidate_sha256": candidate_hash,
                        "flags": FLAGS})
        output = DEST / ("r6-r5-summary-" + corpus.replace("@", "-") + ".json")
        output.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"corpus": corpus, "core": summary["core"],
                          "full": summary["full"], "unmodeled": summary["unmodeled"]}), flush=True)


if __name__ == "__main__":
    main()
