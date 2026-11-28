#!/usr/bin/env python3
"""Compare normal and trace builds over deterministic generated Nexo programs."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from c2_literal_origin_fuzz import narrow_source, varied_source


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(binary, case, trace):
    env = os.environ.copy()
    env.pop("NEXO_ORIGIN_TRACE", None)
    if trace:
        env["NEXO_ORIGIN_TRACE"] = "1"
    proc = subprocess.run(
        [str(binary.resolve()), "check", str(case), "--profile", "f0-0003-b-v1", "--format", "json"],
        capture_output=True, env=env, timeout=30, check=False,
    )
    if proc.returncode not in (0, 1):
        raise ValueError(f"bad exit {proc.returncode}")
    return proc


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--normal", required=True, type=Path)
    parser.add_argument("--trace", required=True, type=Path)
    parser.add_argument("--count", type=int, default=1000)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.count < 1000:
        parser.error("at least 1000 programs are required")
    counts = {"programs": 0, "equal_outputs": 0, "events": 0,
              "finish_events": 0, "null_source_ordinals": 0, "normal_trace_markers": 0}
    with tempfile.TemporaryDirectory(prefix="c2-parity-") as folder:
        case = Path(folder) / "case.n"
        for seed in range(args.count):
            source = varied_source(seed) if seed % 2 == 0 else narrow_source(seed)
            case.write_text(source, encoding="utf-8")
            normal = execute(args.normal, case, False)
            trace_without_env = execute(args.trace, case, False)
            trace_with_env = execute(args.trace, case, True)
            identity = (normal.returncode, normal.stdout)
            if identity != (trace_without_env.returncode, trace_without_env.stdout) or identity != (trace_with_env.returncode, trace_with_env.stdout):
                raise ValueError(f"output changed at seed {seed}")
            if normal.stderr or trace_without_env.stderr:
                raise ValueError(f"unexpected untraced stderr at seed {seed}")
            lines = trace_with_env.stderr.decode("utf-8").splitlines()
            if any(not line.startswith("C2_ORIGIN\t") for line in lines):
                raise ValueError(f"unexpected traced stderr at seed {seed}")
            events = [json.loads(line.removeprefix("C2_ORIGIN\t")) for line in lines]
            if any(event.get("schema") != "c2-origin-jsonl-v2" for event in events):
                raise ValueError(f"wrong trace schema at seed {seed}")
            finishes = [event for event in events if event.get("event") == "finish"]
            counts["programs"] += 1
            counts["equal_outputs"] += 1
            counts["events"] += len(events)
            counts["finish_events"] += len(finishes)
            counts["null_source_ordinals"] += sum(event.get("source_ordinal") is None for event in finishes)
    normal_bytes = args.normal.read_bytes()
    counts["normal_trace_markers"] = normal_bytes.count(b"C2_ORIGIN") + normal_bytes.count(b"c2-origin-jsonl")
    if counts["normal_trace_markers"]:
        raise ValueError("normal binary contains trace markers")
    report = {"schema": "c2-trace-parity-v1", "normal_sha256": sha(args.normal),
              "trace_sha256": sha(args.trace), "counts": counts}
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
