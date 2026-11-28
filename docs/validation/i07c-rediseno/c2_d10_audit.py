#!/usr/bin/env python3
"""Audit D10 against complete oracle signatures in a C2 fuzz report.

This is a read-only audit of evidence, not a family classifier or a gate.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate


def relation_only(main, candidate):
    if main[0] != candidate[0] or len(main[1]) != len(candidate[1]):
        return False
    return main != candidate and all(left[:5] == right[:5]
                                     for left, right in zip(main[1], candidate[1]))


def audit(example):
    derived = gate.oracle.derive(example["source"])
    complete = not any(derived.get(key) for key in
                       ("error", "unmodeled", "no_fijado", "d_optional"))
    main, candidate = example["main"], example["candidate"]
    relation_delta = relation_only(main, candidate)
    oracle_signature = gate.oracle_signature(derived) if complete else None
    exact_oracle = complete and candidate == oracle_signature
    if relation_delta and exact_oracle:
        disposition = "relation_oracle"
    elif relation_delta:
        disposition = "d9_keep_main"
    else:
        disposition = "non_relation_delta_requires_adjudication"
    return {"seed": example["seed"], "source": example["source"],
            "source_sha256": hashlib.sha256(example["source"].encode("ascii")).hexdigest(),
            "class": example["class"], "relation_only": relation_delta,
            "oracle_complete": complete, "candidate_equals_complete_oracle": exact_oracle,
            "disposition": disposition, "main": main, "candidate": candidate,
            "oracle": derived}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    original = args.input.read_bytes()
    fuzz = json.loads(original)
    if fuzz["mode"] != "varied" or fuzz["programs"] != 1000 or not fuzz["unique_sources"] == 1000:
        raise ValueError("expected the 1000-source varied D10 probe")
    rows = [audit(example) for example in fuzz["examples"]
            if example["class"] in ("D", "A-unmodeled")]
    counts = dict(sorted(collections.Counter(row["disposition"] for row in rows).items()))
    report = {"schema": "i07c-c2-d10-audit-v1", "input_sha256": hashlib.sha256(original).hexdigest(),
              "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
              "candidate_gate_sha256": gate.sha(Path(gate.__file__)),
              "rows": rows, "counts": counts}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(counts)


if __name__ == "__main__":
    main()
