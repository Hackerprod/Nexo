#!/usr/bin/env python3
"""Measure the disposable R6 lib-test /MAP pair and 65 KiB probes."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from i07c_rd_drive_r6 import FLAGS

EXE = Path("C:/nexo-work/target-r6/debug/deps/nexo-85c816797ea807f1.exe")
MAP = Path("C:/nexo-work/tmp/r6-libtest.map")
DUMPBIN = Path("C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/dumpbin.exe")
DEST = Path("docs/validation/i07c-rediseno/r6-stack-map.json")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    frame = subprocess.run(
        [sys.executable, "-B", "scripts/f5-frame-sizes.py", str(EXE), str(MAP),
         "--dumpbin", str(DUMPBIN)],
        capture_output=True, text=True, timeout=120,
    )
    if frame.returncode:
        raise RuntimeError(frame.stderr)
    env = os.environ.copy()
    env["NEXO_XV"] = FLAGS
    results = []
    for case in ("option_match126", "result_match126"):
        for repeat in range(1, 6):
            run = subprocess.run(
                [sys.executable, "-B", "scripts/i07c_stack_probe/probe.py", str(EXE),
                 "--mode", "TYPES", "--kib", "65", "--case", case],
                capture_output=True, text=True, timeout=180, env=env,
            )
            results.append({"case": case, "repeat": repeat, "exit": run.returncode,
                            "stdout": run.stdout.strip(), "stderr": run.stderr.strip()})
    document = {"test_exe_sha256": sha(EXE), "map_sha256": sha(MAP),
                "rustc": subprocess.check_output(["rustc", "+1.99.1", "--version"], text=True).strip(),
                "flags": FLAGS, "frame_output": frame.stdout.splitlines(),
                "probes": results,
                "prior_failed_run": {"case": "option_match126", "repeat": 3,
                                     "exit": 3221225725,
                                     "context": "first 5/5 attempt; subsequent isolated repetitions also failed intermittently"}}
    DEST.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"pair": next(line for line in document["frame_output"] if line.startswith("pair=")),
                      "probes": len(results), "failures": sum(row["exit"] != 0 for row in results),
                      "exe_sha256": document["test_exe_sha256"],
                      "map_sha256": document["map_sha256"]}))


if __name__ == "__main__":
    main()
