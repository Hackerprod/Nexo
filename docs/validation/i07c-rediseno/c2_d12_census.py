#!/usr/bin/env python3
"""Apply D12 to the frozen 5,000-source C2 survey without running binaries."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate


def diagnostic(item):
    """D12 identity: code, primary, subcause, and related spans."""
    return (item[0], tuple(item[2]), item[1],
            tuple(tuple(span) if span is not None else None for span in item[5:]))


def bag(signature):
    return collections.Counter(diagnostic(item) for item in signature[1])


def distance(left, right):
    return sum((bag(left) - bag(right)).values()) + sum((bag(right) - bag(left)).values())


def new_false_positives(main, candidate, oracle):
    main_extra = bag(main) - bag(oracle)
    candidate_extra = bag(candidate) - bag(oracle)
    return sum((candidate_extra - main_extra).values())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    report = json.loads(raw)
    counts = collections.Counter()
    rows = []
    for row in report["hard_failures"]:
        if row["reason"] != "unclassified_difference":
            continue
        derived = gate.oracle.derive(row["source"])
        complete = not any(derived.get(key) for key in
                           ("error", "unmodeled", "no_fijado", "d_optional"))
        record = {"seed": row["seed"],
                  "source_sha256": hashlib.sha256(row["source"].encode("ascii")).hexdigest(),
                  "complete": complete}
        if complete:
            expected = gate.oracle_signature(derived)
            dm = distance(row["main"], expected)
            dc = distance(row["candidate"], expected)
            new_fp = new_false_positives(row["main"], row["candidate"], expected)
            accepted = dc < dm and new_fp == 0 and row["main"][0] == row["candidate"][0]
            status = "improvement" if accepted else "complete_defect"
            record.update(distance_main=dm, distance_candidate=dc,
                          new_false_positives=new_fp, status=status)
        else:
            status = "conserve_main"
            record.update(status=status,
                          oracle_limit={key: derived[key] for key in
                                        ("error", "unmodeled", "no_fijado", "d_optional")
                                        if derived.get(key)})
        counts[status] += 1
        rows.append(record)
    result = {"schema": "i07c-c2-d12-census-v1",
              "input_sha256": hashlib.sha256(raw).hexdigest(),
              "candidate_sha256": report["candidate_release_sha256"],
              "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
              "identity": ["code", "primary", "subcause", "relations"],
              "counts": dict(counts), "rows": rows}
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(dict(counts))


if __name__ == "__main__":
    main()
