#!/usr/bin/env python3
"""Replay the frozen 105 new differences and 120 historical C2 rows.

This is an iteration aid. It is not a full 5,000-source fuzz gate.
"""

import argparse
import collections
import json
from pathlib import Path

import candidate_gate as gate
from c2_d12_census import distance, new_false_positives

HERE = Path(__file__).resolve().parent


def compact_signature(compact):
    diagnostics = []
    for item in compact["diagnosticos"]:
        related = {rel["role"]: rel["span"] for rel in item["relations"]}
        diagnostics.append([item["code"], item["subcause"], item["primary"],
                            item["expected_type"] or "", item["found_type"] or "",
                            related.get("expected_type"), related.get("found_type")])
    return [compact["outcome"], diagnostics]


def classify(main, candidate, source):
    if candidate == main:
        return "same"
    derived = gate.oracle.derive(source)
    complete = not any(derived.get(key) for key in
                       ("error", "unmodeled", "no_fijado", "d_optional"))
    if not complete:
        return "conserve_main"
    oracle = gate.oracle_signature(derived)
    if (candidate[0] == main[0] and distance(candidate, oracle) < distance(main, oracle)
            and new_false_positives(main, candidate, oracle) == 0):
        return "improvement"
    return "complete_defect"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    survey = json.loads((HERE / "c2-fuzz-5000-pd6.json").read_text(encoding="utf-8"))
    counts = collections.Counter()
    rows = []
    for row in survey["hard_failures"]:
        if row["reason"] != "unclassified_difference":
            continue
        actual = gate.signature(gate.execute(args.candidate_exe, row["source"],
                                             survey["profile"], args.case))
        status = classify(row["main"], actual, row["source"])
        counts[status] += 1
        rows.append({"seed": row["seed"], "status": status})
    iteration = json.loads((HERE / "c2-final-iteration-v15f.json").read_text(encoding="utf-8"))
    historical_changed = []
    for row_id, record in iteration["rows"].items():
        actual = gate.signature(gate.execute(args.candidate_exe, record["source"],
                                             record["profile"], args.case))
        expected = compact_signature(iteration["results"][row_id]["candidate"])
        if actual != expected:
            historical_changed.append(row_id)
    result = {"schema": "i07c-c2-d12-replay-v1",
              "candidate_sha256": gate.sha(args.candidate_exe),
              "survey_sha256": gate.sha(HERE / "c2-fuzz-5000-pd6.json"),
              "counts": dict(counts), "rows": rows,
              "historical_changed": historical_changed}
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print(dict(counts), "historical_changed", len(historical_changed))


if __name__ == "__main__":
    main()
