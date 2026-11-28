#!/usr/bin/env python3
"""Run the complete immutable C2 gate and discharge only causal family members."""

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--previous", type=Path, required=True)
    parser.add_argument("--cross-report", type=Path, required=True)
    parser.add_argument("--family-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    previous = json.loads(args.previous.read_text(encoding="utf-8"))
    if not previous["complete"] or previous["commit"] != "C1":
        raise ValueError("--previous must be the complete C1 gate")
    if previous["main_sha256"] != sha(args.main_exe):
        raise ValueError("main binary differs from C1 gate")
    if previous["classifier_sha256"] != sha(HERE / "candidate_gate.py"):
        raise ValueError("immutable candidate_gate.py differs from C1")
    iteration_path = HERE / "c2-final-iteration-v15f.json"
    trace_path = HERE / "c2-final-trace-v15f.json.gz"
    iteration = json.loads(iteration_path.read_text(encoding="utf-8"))
    if not iteration["complete"] or iteration["main_sha256"] != sha(args.main_exe):
        raise ValueError("frozen iteration main is stale")
    if iteration["candidate_sha256"] != sha(args.candidate_exe):
        raise ValueError("frozen iteration candidate is stale")
    common = ["--main-exe", str(args.main_exe), "--candidate-exe", str(args.candidate_exe),
              "--case", str(args.case)]
    command = [sys.executable, "-B", str(HERE / "candidate_gate.py"), *common,
               "--commit", "C2", "--previous", str(args.previous),
               "--output", str(args.cross_report)]
    run = subprocess.run(command, check=False)
    if run.returncode not in (0, 1) or not args.cross_report.exists():
        raise RuntimeError("immutable complete gate did not produce a report")
    cross = json.loads(args.cross_report.read_text(encoding="utf-8"))
    if not cross["complete"] or cross["commit"] != "C2":
        raise ValueError("incomplete or wrong cross gate")
    if cross["main_sha256"] != sha(args.main_exe) or cross["candidate_sha256"] != sha(args.candidate_exe):
        raise ValueError("cross gate used different binaries")
    if len(cross["corpora"]) != 20 or len(cross["class_by_row"]) != 151188:
        raise ValueError("cross gate did not cover 20 corpus and 151188 rows")
    if sum(corpus["rows"] for corpus in cross["corpora"].values()) != 151188:
        raise ValueError("cross gate row counts mismatch")
    family_cmd = [sys.executable, "-B", str(HERE / "c2_family_check.py"),
                  "--iteration", str(iteration_path), "--trace", str(trace_path),
                  "--output", str(args.family_report)]
    subprocess.run(family_cmd, check=True)
    families = json.loads(args.family_report.read_text(encoding="utf-8"))
    if families["unresolved"] or len(families["members"]) != 64:
        raise ValueError("causal family audit is incomplete")
    failures = []
    members = families["members"]
    cross_failures = []
    for failure in cross["failures"]:
        row_id, label = failure.rsplit("=", 1) if "=" in failure else (failure, "")
        if label in ("A-unmodeled", "D") and row_id in members:
            if cross["class_by_row"][row_id] != iteration["results"][row_id]["class"]:
                failures.append(row_id + "=class-changed")
            cross_failures.append(row_id)
        else:
            failures.append(failure)
    if set(cross_failures) != set(members) or len(cross_failures) != len(members):
        failures.append("cross-failure-membership-mismatch")
    report = {"schema": "i07c-c2-cross-gate-v1", "complete": True, "commit": "C2",
              "main_sha256": sha(args.main_exe), "candidate_sha256": sha(args.candidate_exe),
              "candidate_gate_sha256": sha(HERE / "candidate_gate.py"),
              "family_checker_sha256": sha(HERE / "c2_family_check.py"),
              "previous_sha256": sha(args.previous), "cross_report_sha256": sha(args.cross_report),
              "family_report_sha256": sha(args.family_report),
              "corpora": 20, "rows": 151188,
              "family_counts": {name: data["count"] for name, data in families["families"].items()},
              "failures": failures}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print("C2 cross gate", report["rows"], "rows; family members", len(members),
          "; failures", len(failures))
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
