#!/usr/bin/env python3
"""Cross-check complete C1/C2 reports and D13's historical-member census."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_text_sha(path):
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--previous", type=Path, required=True)
    parser.add_argument("--cross-report", type=Path, required=True)
    parser.add_argument("--iteration", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prior, cross, iteration = map(read, (args.previous, args.cross_report, args.iteration))
    manifest_path = HERE / "c2-min-family-members-v15f.json"
    manifest = read(manifest_path)
    failures = []
    for name, report, commit in (("previous", prior, "C1"), ("cross", cross, "C2")):
        if not report["complete"] or report["commit"] != commit or report["failures"]:
            failures.append(f"{name}: incomplete, wrong commit, or failures")
        if len(report["corpora"]) != 20 or len(report["class_by_row"]) != 151188:
            failures.append(f"{name}: missing corpus rows")
        if sum(item["rows"] for item in report["corpora"].values()) != 151188:
            failures.append(f"{name}: wrong corpus row sum")
        if report["classifier_sha256"] != sha(HERE / "candidate_gate.py"):
            failures.append(f"{name}: classifier SHA changed")
    if cross["main_sha256"] != prior["main_sha256"] or cross[
        "oracle_sha256"] != prior["oracle_sha256"]:
        failures.append("C1/C2 baseline or oracle differs")
    if not iteration["complete"] or len(iteration["rows"]) != 120:
        failures.append("historical iteration incomplete")
    if iteration["main_sha256"] != cross["main_sha256"] or iteration[
        "candidate_sha256"] != cross["candidate_sha256"]:
        failures.append("historical iteration used different binaries")
    actual_members = sorted(row_id for row_id, row in iteration["rows"].items()
                            if row["candidate"] != row["main"])
    if (iteration["members"] != actual_members or actual_members or
            manifest["members"] != actual_members or manifest["rows_checked"] != 120 or
            manifest["candidate_sha256"] != iteration["candidate_sha256"] or
            manifest["iteration_sha256"] != git_text_sha(args.iteration)):
        failures.append("historical members need adjudication")
    if any(prior["class_by_row"][row] == "B" and label == "C"
           for row, label in cross["class_by_row"].items()):
        failures.append("C1 oracle repair returned to main")
    counts = dict(collections.Counter(cross["class_by_row"].values()))
    report = {"schema": "i07c-c2-min-cross-gate-v1", "complete": True,
              "previous_sha256": git_text_sha(args.previous),
              "cross_sha256": git_text_sha(args.cross_report),
              "iteration_sha256": git_text_sha(args.iteration),
              "manifest_sha256": git_text_sha(manifest_path),
              "classifier_sha256": sha(HERE / "candidate_gate.py"),
              "main_sha256": cross["main_sha256"],
              "candidate_sha256": cross["candidate_sha256"],
              "corpora": 20, "rows": 151188, "classes": counts,
              "historical_members": len(actual_members), "failures": failures}
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("C2-min cross", report["rows"], "rows; historical members",
          len(actual_members), "; failures", len(failures))
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
