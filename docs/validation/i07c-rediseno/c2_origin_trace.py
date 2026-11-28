#!/usr/bin/env python3
"""Collect read-only C2 solver traces; this is not an ADR classifier.

Build the candidate with `cargo build --release --features c2-origin-trace`.
The trace is emitted only when NEXO_ORIGIN_TRACE is set. Normal CLI JSON and
diagnostics must still match the frozen P-B observation for the selected rows.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

import candidate_gate as gate

HERE = Path(__file__).resolve().parent
FROZEN = HERE / "c2-d8-iteration-pb.json"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--row", action="append", required=True,
                        help="Corpus row ID; repeat to trace several rows")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    frozen = json.loads(FROZEN.read_text(encoding="utf-8"))
    observations = {}
    for row_id in args.row:
        if row_id not in frozen["rows"]:
            parser.error(f"row absent from frozen iteration: {row_id}")
        row = frozen["rows"][row_id]
        if digest(row["source"].encode("utf-8")) != row["source_sha256"]:
            raise ValueError(f"source SHA mismatch: {row_id}")
        with tempfile.TemporaryDirectory(prefix="c2-origin-") as folder:
            case = Path(folder) / "case.n"
            case.write_bytes(row["source"].encode("utf-8"))
            env = os.environ.copy()
            env["NEXO_ORIGIN_TRACE"] = "1"
            run = subprocess.run(
                [str(args.binary.resolve()), "check", str(case), "--profile",
                 row["profile"], "--format", "json"],
                capture_output=True, env=env, timeout=30, check=False,
            )
        if run.returncode not in (0, 1):
            raise RuntimeError(f"{row_id}: exit {run.returncode}: {run.stderr[:300]!r}")
        output = gate.compact_match(json.loads(run.stdout))
        if output != frozen["results"][row_id]["candidate"]:
            raise ValueError(f"trace feature changed candidate output: {row_id}")
        stderr = run.stderr.decode("utf-8", errors="replace").splitlines()
        unexpected = [line for line in stderr if not line.startswith("C2_ORIGIN\t")]
        if unexpected:
            raise RuntimeError(f"{row_id}: unrelated stderr: {unexpected[:3]}")
        escaped_case = json.dumps(str(case), ensure_ascii=True)[1:-1]
        trace = [line.removeprefix("C2_ORIGIN\t").replace(escaped_case, "<SOURCE>")
                 for line in stderr]
        if any(str(case) in line or escaped_case in line for line in trace):
            raise ValueError(f"{row_id}: unnormalized source path")
        observations[row_id] = {
            "source_sha256": row["source_sha256"],
            "profile": row["profile"],
            "exit": run.returncode,
            "trace": trace,
            "candidate": output,
        }

    report = {
        "schema": "i07c-c2-origin-trace-v1",
        "frozen_sha256": gate.sha(FROZEN),
        "binary_sha256": gate.sha(args.binary),
        "rows": observations,
    }
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n",
                           encoding="utf-8")
    print(f"{len(observations)} rows, "
          f"{sum(len(row['trace']) for row in observations.values())} trace events")


if __name__ == "__main__":
    main()
