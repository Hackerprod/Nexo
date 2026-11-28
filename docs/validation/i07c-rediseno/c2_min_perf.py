#!/usr/bin/env python3
"""Measure a fixed 4,000-pair program on main and D13 C2 minimum."""

import argparse
import hashlib
import json
import statistics
import subprocess
import time
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def program():
    return "fn Main(){" + "".join(f"let a{i}=[1];let b{i}=a{i};" for i in range(4000)) + "}"


def run(binary, verb, case):
    started = time.perf_counter()
    result = subprocess.run([str(binary), verb, str(case), "--profile", "f0-0003-b-v1",
                             "--format", "json"], capture_output=True, timeout=120, check=False)
    elapsed = time.perf_counter() - started
    if result.returncode not in (0, 1):
        raise RuntimeError(f"{binary} {verb}: exit {result.returncode}: {result.stderr[:300]!r}")
    output = json.loads(result.stdout)
    if output["outcome"]["kind"] in ("driver_error", "internal_error"):
        raise RuntimeError(f"{binary} {verb}: hard outcome")
    signature = [output["outcome"]["kind"], [(d["code"], d.get("subcause"),
                  d["primary_span"]["start"], d["primary_span"]["end"])
                 for d in output["diagnostics"]]]
    return elapsed, signature


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = program()
    args.case.write_bytes(source.encode("ascii"))
    seconds = {}
    ratios = {}
    for verb in ("check", "run"):
        times = {"main": [], "C2": []}
        expected = None
        for _ in range(3):
            for name, binary in (("main", args.main_exe), ("C2", args.candidate_exe)):
                elapsed, signature = run(binary, verb, args.case)
                if expected is None:
                    expected = signature
                elif signature != expected:
                    raise RuntimeError(f"{verb}: main/C2 result differs")
                times[name].append(elapsed)
        seconds[verb] = times
        ratios[verb] = statistics.median(times["C2"]) / statistics.median(times["main"])
    report = {"schema": "i07c-c2-min-perf-v1", "complete": True,
              "source_bytes": len(source),
              "source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(),
              "pairs": 4000, "profile": "f0-0003-b-v1",
              "binary_sha256": {"main": sha(args.main_exe), "C2": sha(args.candidate_exe)},
              "runner_sha256": sha(Path(__file__)), "seconds": seconds,
              "median_ratio": ratios}
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("perf ratios", ratios)
    if any(ratio > 3 for ratio in ratios.values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
