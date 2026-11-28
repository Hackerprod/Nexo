#!/usr/bin/env python3
"""Classify a frozen C2 fuzz report with D11's complete-oracle rule."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.input.read_bytes()
    report = json.loads(source)
    counts = collections.Counter()
    rows = []
    for row in report["hard_failures"]:
        if row["reason"] != "unclassified_difference":
            continue
        oracle = gate.oracle.derive(row["source"])
        complete = not any(oracle.get(key) for key in
                           ("error", "unmodeled", "no_fijado", "d_optional"))
        expected = gate.oracle_signature(oracle) if complete else None
        if complete:
            status = ("accepted" if row["candidate"] == expected
                      and row["main"] != expected else "complete_defect")
        else:
            status = "unmodeled_defect"
        counts[status] += 1
        rows.append({"seed": row["seed"], "source_sha256": hashlib.sha256(
            row["source"].encode("ascii")).hexdigest(), "status": status,
                     "main_equals_oracle": complete and row["main"] == expected,
                     "candidate_equals_oracle": complete and row["candidate"] == expected,
                     "oracle_limit": {key: oracle[key] for key in
                                      ("error", "unmodeled", "no_fijado", "d_optional")
                                      if oracle.get(key)}})
    result = {"schema": "i07c-c2-d11-census-v1",
              "input_sha256": hashlib.sha256(source).hexdigest(),
              "candidate_sha256": report["candidate_release_sha256"],
              "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
              "counts": dict(counts), "rows": rows}
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print(dict(counts))


if __name__ == "__main__":
    main()
