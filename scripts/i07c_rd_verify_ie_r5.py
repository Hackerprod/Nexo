#!/usr/bin/env python3
"""Re-run B's 21 match/assignment internal-error sources against one binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from i07c_rd_drive_r6 import FLAGS


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("exe", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    exe = args.exe.resolve()
    fixture = Path("docs/validation/i07c-rediseno/r6-driver-error-audit.json")
    cases = json.loads(fixture.read_text(encoding="utf-8"))["rows"]
    env = os.environ.copy()
    env["NEXO_XV"] = FLAGS
    observed = []
    with tempfile.TemporaryDirectory(dir="C:/nexo-work/tmp") as directory:
        for case in cases:
            path = Path(directory) / "source.n"
            path.write_bytes(case["source"].encode("ascii"))
            row = {"corpus": case["corpus"], "index": case["index"],
                   "source_sha256": case["source_sha256"], "commands": {}}
            for command in ("check", "run"):
                proc = subprocess.run(
                    [str(exe), command, "--format", "json", "--profile", case["profile"], str(path)],
                    env=env, capture_output=True, text=True, timeout=60)
                try:
                    payload = json.loads(proc.stdout)
                    kind = (payload.get("outcome") or {}).get("kind")
                    row["commands"][command] = {
                        "exit": proc.returncode, "kind": kind,
                        "driver_error": payload.get("driver_error"),
                        "internal_error": payload.get("internal_error"),
                        "diagnostics": [(d["code"], d["primary_span"]["start"],
                                         d["primary_span"]["end"])
                                        for d in payload.get("diagnostics", [])],
                    }
                except (KeyError, ValueError, TypeError) as error:
                    row["commands"][command] = {"error": str(error), "exit": proc.returncode,
                                                "stdout": proc.stdout[-300:], "stderr": proc.stderr[-300:]}
            observed.append(row)
    report = {"binary_sha256": hashlib.sha256(exe.read_bytes()).hexdigest(),
              "flags": FLAGS, "rows": observed}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    abnormal = [(r["corpus"], r["index"], cmd) for r in observed for cmd, entry in r["commands"].items()
                if entry.get("error") or entry.get("driver_error") is not None
                or entry.get("internal_error") is not None
                or entry.get("kind") not in ("checked", "rejected", "unit", "simulator_fault")]
    print(json.dumps({"rows": len(observed), "commands": 2 * len(observed),
                      "abnormal": abnormal}, ensure_ascii=False))
    if abnormal:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
