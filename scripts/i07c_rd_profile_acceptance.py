#!/usr/bin/env python3
"""Check acceptance parity for frozen directed sources in three profiles."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

import i07c_rd_sweep as sweep
from i07c_rd_b_minima import CASES
from i07c_rd_cheap import MINIMA
from i07c_rd_drive6 import BASE, CANDIDATE, FLAGS


def inspect(binary, command, profile, source, flags):
    with tempfile.TemporaryDirectory(prefix="i07c-rd-accept-") as directory:
        path = Path(directory) / "p.n"
        path.write_bytes(source.encode("utf-8"))
        env = os.environ.copy()
        env.pop("NEXO_XV", None)
        if flags:
            env["NEXO_XV"] = flags
        with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stdout, \
             tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stderr:
            child = subprocess.Popen(
                [binary, command, "--format", "json", "--profile", profile, str(path)],
                stdout=stdout, stderr=stderr, env=env,
            )
            last_output = time.monotonic()
            last_size = 0
            while child.poll() is None:
                size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
                if size != last_size:
                    last_output, last_size = time.monotonic(), size
                if sweep.working_set(child.pid) > sweep.MAX_WORKING_SET:
                    child.kill()  # This invocation created this PID.
                    child.wait()
                    raise RuntimeError(f"INCONCLUSO >6 GB: {command} {profile}")
                if time.monotonic() - last_output > sweep.SILENCE_LIMIT:
                    child.kill()  # This invocation created this PID.
                    child.wait()
                    raise RuntimeError(f"INCONCLUSO >20 min silent: {command} {profile}")
                time.sleep(0.02)
            stdout.seek(0)
            stderr.seek(0)
            output, error = stdout.read(), stderr.read()
    if error:
        raise RuntimeError(f"INCONCLUSO stderr: {error[-300:]}")
    document = json.loads(output)
    outcome = document["outcome"]["kind"]
    return {"exit": child.returncode, "outcome": outcome,
            "accepted": outcome in ("checked", "unit")}


def main():
    sources = {**MINIMA, **{name: source for name, source, _ in CASES}}
    profiles = ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1")
    rows = []
    for profile in profiles:
        for name, source in sources.items():
            readings = {}
            for label, binary, flags in (("main", BASE, None), ("r5", CANDIDATE, FLAGS)):
                readings[label] = {command: inspect(binary, command, profile, source, flags)
                                   for command in ("check", "run")}
            main_accept = readings["main"]["check"]["accepted"]
            r5_accept = readings["r5"]["check"]["accepted"]
            rows.append({"profile": profile, "name": name,
                         "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                         "readings": readings,
                         "check_run_same": all(readings[label]["check"]["accepted"] ==
                                               readings[label]["run"]["accepted"]
                                               for label in ("main", "r5")),
                         "main_r5_same": main_accept == r5_accept})
    result = {"main_sha256": hashlib.sha256(Path(BASE).read_bytes()).hexdigest(),
              "r5_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
              "flags": FLAGS, "rows": rows,
              "check_run_failures": [r["profile"] + "/" + r["name"] for r in rows
                                     if not r["check_run_same"]],
              "main_r5_failures": [r["profile"] + "/" + r["name"] for r in rows
                                   if not r["main_r5_same"]]}
    dest = Path("docs/validation/i07c-rediseno/r5-profile-acceptance.json")
    dest.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"rows": len(rows),
                      "check_run_failures": result["check_run_failures"],
                      "main_r5_failures": result["main_r5_failures"]}))


if __name__ == "__main__":
    main()
