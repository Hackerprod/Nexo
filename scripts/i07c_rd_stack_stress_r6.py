#!/usr/bin/env python3
"""Repeat the 65 KiB stack probes and retain every abnormal exit."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from i07c_rd_drive_r6 import FLAGS


EXE = Path("C:/nexo-work/target-r6/debug/deps/nexo-85c816797ea807f1.exe")


def main():
    batches = []
    for case, flags, count in (("option_match126", FLAGS, 100),
                               ("option_match126", None, 100),
                               ("result_match126", FLAGS, 30)):
        env = os.environ.copy()
        env.pop("NEXO_XV", None)
        if flags:
            env["NEXO_XV"] = flags
        failures = []
        for repeat in range(1, count + 1):
            result = subprocess.run([sys.executable, "-B", "scripts/i07c_stack_probe/probe.py",
                                     str(EXE), "--mode", "TYPES", "--kib", "65", "--case", case],
                                    capture_output=True, text=True, timeout=180, env=env)
            if result.returncode:
                failures.append({"repeat": repeat, "exit": result.returncode,
                                 "stdout": result.stdout.strip(), "stderr": result.stderr.strip()})
        batches.append({"case": case, "flags": flags, "count": count,
                        "successes": count - len(failures), "failures": failures})
        print(case, "flags" if flags else "unflagged", count - len(failures), count, flush=True)
    document = {"test_exe_sha256": hashlib.sha256(EXE.read_bytes()).hexdigest(),
                "batches": batches}
    output = Path("docs/validation/i07c-rediseno/r6-stack-stress.json")
    output.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
