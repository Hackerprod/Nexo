#!/usr/bin/env python3
"""Count planning passes of the disposable prototype with child watchdogs."""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time

import i07c_rd_sweep as sweep
from i07c_rd_drive_r6 import FLAGS

LOCAL = threading.local()


def one(args):
    exe, corpus, row = args
    if not hasattr(LOCAL, "directory"):
        LOCAL.directory = tempfile.TemporaryDirectory(prefix="i07c-rd-passes-")
    source = Path(LOCAL.directory.name) / "source.n"
    source.write_bytes(sweep.source(row).encode("utf-8"))
    env = dict(os.environ, NEXO_XV=FLAGS + ",passes")
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stdout, tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stderr:
        proc = subprocess.Popen([exe, "check", "--format", "json", "--profile", sweep.profile(corpus), str(source)],
                                stdout=stdout, stderr=stderr, env=env)
        last_output, size = time.monotonic(), 0
        reason = None
        while proc.poll() is None:
            now_size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
            if now_size != size:
                last_output, size = time.monotonic(), now_size
            if sweep.working_set(proc.pid) > sweep.MAX_WORKING_SET:
                reason = "working_set_over_6gb"
                break
            if time.monotonic() - last_output > sweep.SILENCE_LIMIT:
                reason = "silent_over_20min"
                break
            time.sleep(0.02)
        if reason:
            proc.kill()  # Own PID only.
            proc.wait()
            return {"inconclusive": reason, "pid": proc.pid}
        stdout.seek(0)
        stderr.seek(0)
        out, err = stdout.read(), stderr.read()
    try:
        outcome = json.loads(out)["outcome"]
    except (ValueError, KeyError, TypeError):
        return {"inconclusive": "unparseable_output", "exit": proc.returncode, "stderr": err[-200:]}
    if outcome.get("kind") == "driver_error":
        return {"driver_error": outcome.get("reason"), "exit": proc.returncode,
                "stderr": err[-200:]}
    if proc.returncode not in (0, 1) or "InvalidIr" in out + err or "internal_error" in out + err:
        return {"inconclusive": "abnormal_result", "exit": proc.returncode, "stderr": err[-200:]}
    try:
        json.loads(out)
        values = [int(line.split()[1]) for line in err.splitlines() if line.startswith("PASSES ")]
        return max(values, default=0)
    except (ValueError, IndexError):
        return {"inconclusive": "unparseable_output", "stderr": err[-200:]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus")
    parser.add_argument("exe")
    args = parser.parse_args()
    rows = sweep.data(args.corpus)
    exe = str(Path(args.exe).resolve())
    with open(exe, "rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(one, ((exe, args.corpus, row) for row in rows)))
    inconclusive = [(i, value) for i, value in enumerate(results)
                    if isinstance(value, dict) and "inconclusive" in value]
    driver_errors = [(i, value) for i, value in enumerate(results)
                     if isinstance(value, dict) and "driver_error" in value]
    histogram = Counter(value for value in results if isinstance(value, int))
    doc = {"corpus": args.corpus, "rows": len(rows), "binary_sha256": digest,
           "flags": FLAGS + ",passes",
           "max_passes": max(histogram, default=None), "histogram": sorted(histogram.items()),
           "inconclusive": inconclusive, "driver_errors": driver_errors}
    path = Path("docs/validation/i07c-rediseno") / f"r6-pasadas-{args.corpus}.json"
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"completed={path} max={doc['max_passes']} inconclusive={len(inconclusive)} driver_errors={len(driver_errors)} histogram={doc['histogram']}", flush=True)
    if inconclusive:
        raise RuntimeError("INCONCLUSIVE: repeat own failed children")


if __name__ == "__main__":
    main()
