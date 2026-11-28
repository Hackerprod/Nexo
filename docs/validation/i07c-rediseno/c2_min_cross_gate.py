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
    parser.add_argument("--survey", type=Path, required=True)
    parser.add_argument("--varied", type=Path, required=True)
    parser.add_argument("--b-fuzz", type=Path, required=True)
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
    parallel_sha = sha(HERE / "candidate_gate_parallel.py")
    if prior.get("runner_sha256") != parallel_sha or cross.get("runner_sha256") != parallel_sha:
        failures.append("C1/C2 parallel runner SHA changed")
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
    d12_reports = {"survey": read(args.survey), "varied": read(args.varied),
                   "b_fuzz": read(args.b_fuzz)}
    debug_sha = d12_reports["survey"].get("candidate_debug_sha256")
    for name, value in d12_reports.items():
        if (not value.get("complete") or value.get("hard_failures") or
                set(value.get("counts", {})) - {"same", "improvement"}):
            failures.append(f"{name}: D12 defect or incomplete fuzz")
        if (value.get("main_sha256") != cross["main_sha256"] or
                value.get("candidate_sha256") != cross["candidate_sha256"] or
                value.get("candidate_debug_sha256") != debug_sha):
            failures.append(f"{name}: binary SHA changed")
    if (d12_reports["survey"].get("unique_sources") != 5000 or
            not d12_reports["survey"].get("survey") or
            d12_reports["survey"].get("runner_sha256") != sha(HERE / "c2_min_fuzz.py")):
        failures.append("survey: wrong universe or D12 classifier")
    if (d12_reports["varied"].get("unique_sources", 0) < 60000 or
            d12_reports["varied"].get("survey") or
            d12_reports["varied"].get("runner_sha256") != sha(HERE / "c2_min_fuzz.py")):
        failures.append("varied: wrong universe or D12 classifier")
    b_report = d12_reports["b_fuzz"]
    if (b_report.get("unique_sources", 0) < 15000 or
            b_report.get("profile_cases", 0) < 15000 or
            b_report.get("runner_sha256") != sha(HERE / "c2_min_b_fuzz.py")):
        failures.append("B: wrong universe or D12 runner")
    for filename, expected in b_report.get("corpus_sha256", {}).items():
        if sha(HERE / filename) != expected:
            failures.append(f"B: source corpus changed: {filename}")
    report = {"schema": "i07c-c2-min-cross-gate-v1", "complete": True,
              "previous_sha256": git_text_sha(args.previous),
              "cross_sha256": git_text_sha(args.cross_report),
              "iteration_sha256": git_text_sha(args.iteration),
              "manifest_sha256": git_text_sha(manifest_path),
              "d12_report_sha256": {name: git_text_sha(path) for name, path in
                                      (("survey", args.survey), ("varied", args.varied),
                                       ("b_fuzz", args.b_fuzz))},
              "d12_classifier_sha256": sha(HERE / "c2_min_fuzz.py"),
              "b_runner_sha256": sha(HERE / "c2_min_b_fuzz.py"),
              "cross_runner_sha256": sha(Path(__file__)),
              "classifier_sha256": sha(HERE / "candidate_gate.py"),
              "parallel_runner_sha256": parallel_sha,
              "main_sha256": cross["main_sha256"],
              "candidate_sha256": cross["candidate_sha256"],
              "candidate_debug_sha256": debug_sha,
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
