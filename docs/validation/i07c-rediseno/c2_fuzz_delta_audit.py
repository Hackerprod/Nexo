#!/usr/bin/env python3
"""Read-only census of unresolved C2 fuzz deltas against the complete oracle."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate


def diagnostic_key(item):
    return item[0], item[2]


def delta(main, candidate):
    old, new = list(main[1]), list(candidate[1])
    for item in list(old):
        if item in new:
            old.remove(item)
            new.remove(item)
    return old, new


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = json.loads(args.input.read_text(encoding="utf-8"))
    census = collections.Counter()
    rows = []
    for row in report["hard_failures"]:
        if row["reason"] != "unclassified_difference":
            continue
        derived = gate.oracle.derive(row["source"])
        complete = not any(derived.get(key) for key in
                           ("error", "unmodeled", "no_fijado", "d_optional"))
        expected = gate.oracle_signature(derived) if complete else None
        removed, added = delta(row["main"], row["candidate"])
        oracle_removed, oracle_added = delta(row["main"], expected) if complete else ([], [])
        exact_local_keys = complete and (
            sorted(map(diagnostic_key, removed)) == sorted(map(diagnostic_key, oracle_removed))
            and sorted(map(diagnostic_key, added)) == sorted(map(diagnostic_key, oracle_added)))
        exact_local = complete and removed == oracle_removed and added == oracle_added
        oracle_items = expected[1] if complete else []
        local_improvement = complete and all(item in oracle_items for item in added) and all(
            item not in oracle_items for item in removed)
        census["complete" if complete else "incomplete"] += 1
        if exact_local_keys:
            census["exact_local_code_primary"] += 1
        if exact_local:
            census["exact_local_full"] += 1
        if local_improvement:
            census["local_improvement_full_diagnostic"] += 1
        codes = tuple(sorted(collections.Counter(item[0] for item in added).items()))
        losses = tuple(sorted(collections.Counter(item[0] for item in removed).items()))
        census[f"delta:{losses}->{codes}"] += 1
        rows.append({"seed": row["seed"], "source": row["source"],
                     "source_sha256": hashlib.sha256(row["source"].encode("ascii")).hexdigest(),
                     "complete": complete,
                     "exact_local_code_primary": exact_local_keys,
                     "exact_local_full": exact_local,
                     "local_improvement_full_diagnostic": local_improvement,
                     "removed": removed, "added": added,
                     "oracle_removed": oracle_removed, "oracle_added": oracle_added})
    args.output.write_text(json.dumps({
        "schema": "i07c-c2-fuzz-delta-audit-v1",
        "input_sha256": hashlib.sha256(args.input.read_bytes()).hexdigest(),
        "candidate_sha256": report["candidate_release_sha256"],
        "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
        "counts": dict(census), "rows": rows}, indent=2) + "\n",
                           encoding="utf-8")
    print({key: census[key] for key in ("complete", "incomplete",
                                        "local_improvement_full_diagnostic")})


if __name__ == "__main__":
    main()
