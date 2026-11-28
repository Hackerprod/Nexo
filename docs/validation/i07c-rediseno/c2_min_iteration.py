#!/usr/bin/env python3
"""Re-enumerate all 120 historical C2 sources on the D13 minimum."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import c2_origin_iteration as origin
import candidate_gate as gate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    baseline, rows, _ = origin.inventory()
    result = {"schema": "i07c-c2-min-iteration-v1", "complete": True,
              "base_report_sha256": gate.sha(origin.FROZEN),
              "main_sha256": gate.sha(args.main_exe),
              "candidate_sha256": gate.sha(args.candidate_exe),
              "rows": {}, "members": [], "class_counts": {}}
    counts = collections.Counter()
    for row_id, row in rows.items():
        source = row["source"]
        assert hashlib.sha256(source.encode("ascii")).hexdigest() == row["source_sha256"]
        main_output = gate.execute(args.main_exe, source, row["profile"], args.case)
        candidate = gate.execute(args.candidate_exe, source, row["profile"], args.case)
        cls = gate.classify(main_output, candidate, gate.oracle.derive(source))
        counts[cls] += 1
        if gate.signature(candidate) != gate.signature(main_output):
            result["members"].append(row_id)
        result["rows"][row_id] = {"source_sha256": row["source_sha256"],
                                  "profile": row["profile"], "class": cls,
                                  "main": gate.compact_match(main_output),
                                  "candidate": gate.compact_match(candidate)}
    result["class_counts"] = dict(counts)
    result["members"].sort()
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("historical rows", len(rows), "members", len(result["members"]),
          "classes", result["class_counts"])


if __name__ == "__main__":
    main()
