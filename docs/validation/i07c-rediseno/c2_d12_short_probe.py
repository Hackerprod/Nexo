#!/usr/bin/env python3
"""Measure frozen 62 D12 defects and ten improvements on one binary.

This is an iteration probe. It does not replace fuzz or the cross gate.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate
import c2_d12_census as d12
from c2_commit_bisect import defect_status

HERE = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    frozen = json.loads((HERE / "c2-fuzz-5000-pd6.json").read_text(encoding="utf-8"))
    census = json.loads((HERE / "c2-d12-census-5000.json").read_text(encoding="utf-8"))
    by_seed = {row["seed"]: row for row in census["rows"]}
    improvements = [row["seed"] for row in census["rows"] if row["status"] == "improvement"][:10]
    wanted = {row["seed"] for row in census["rows"] if row["status"] != "improvement"}
    wanted.update(improvements)
    counts = collections.Counter()
    rows = []
    for row in frozen["hard_failures"]:
        seed = row["seed"]
        if seed not in wanted:
            continue
        candidate = gate.signature(gate.execute(args.candidate_exe, row["source"],
                                                frozen["profile"], args.case))
        complete = by_seed[seed]["complete"]
        expected = gate.oracle_signature(gate.oracle.derive(row["source"])) if complete else None
        status = defect_status(row["main"], candidate, expected, complete)
        counts[status] += 1
        rows.append({"seed": seed, "status": status, "final_status": by_seed[seed]["status"],
                     "candidate": candidate})
    assert len(rows) == 72, len(rows)
    report = {"schema": "i07c-c2-d12-short-probe-v1", "complete": False,
              "candidate_sha256": hashlib.sha256(args.candidate_exe.read_bytes()).hexdigest(),
              "counts": dict(counts), "rows": rows}
    args.output.write_bytes((json.dumps(report, indent=2, ensure_ascii=True) + "\n").encode("utf-8"))
    print(dict(counts))


if __name__ == "__main__":
    main()
