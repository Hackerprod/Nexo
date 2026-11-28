#!/usr/bin/env python3
"""Read-only census of unresolved C2 fuzz deltas against the complete oracle."""

import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import subprocess

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


def trace_second_join(binary, case_path, row, added):
    case_path.write_bytes(row["source"].encode("ascii"))
    environment = os.environ.copy()
    environment["NEXO_ORIGIN_TRACE"] = "1"
    run = subprocess.run([str(binary), "check", str(case_path), "--profile",
                          "f0-0003-b-v1", "--format", "json"], capture_output=True,
                         env=environment, timeout=30, check=False)
    if run.returncode not in (0, 1) or gate.signature(json.loads(run.stdout)) != row["candidate"]:
        raise ValueError(f"trace parity failed at seed {row['seed']}")
    lines = run.stderr.decode("utf-8").splitlines()
    if any(not line.startswith("C2_ORIGIN\t") for line in lines):
        raise ValueError(f"unexpected trace stderr at seed {row['seed']}")
    joins = [json.loads(line.removeprefix("C2_ORIGIN\t")) for line in lines]
    joins = [event for event in joins if event.get("event") == "if_join_context"
             and not event.get("external_context")]
    return [any(item[0] == "NX-E1303" and event.get("second_value") == item[2]
                and event.get("first_value") == item[5] for event in joins)
            for item in added]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--trace-exe", type=Path)
    parser.add_argument("--case", type=Path, help="writable source path for trace executions")
    args = parser.parse_args()
    if bool(args.trace_exe) != bool(args.case):
        parser.error("--trace-exe and --case must be provided together")
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
        join_matches = trace_second_join(args.trace_exe, args.case, row, added) if args.trace_exe else []
        if args.trace_exe:
            group = ("complete_local" if local_improvement else
                     "complete_other" if complete else "incomplete")
            census[group + "_added_e1303"] += sum(item[0] == "NX-E1303" for item in added)
            census[group + "_join_match"] += sum(join_matches)
        codes = tuple(sorted(collections.Counter(item[0] for item in added).items()))
        losses = tuple(sorted(collections.Counter(item[0] for item in removed).items()))
        census[f"delta:{losses}->{codes}"] += 1
        audit_row = {"seed": row["seed"], "source": row["source"],
                     "source_sha256": hashlib.sha256(row["source"].encode("ascii")).hexdigest(),
                     "complete": complete,
                     "exact_local_code_primary": exact_local_keys,
                     "exact_local_full": exact_local,
                     "local_improvement_full_diagnostic": local_improvement,
                     "removed": removed, "added": added,
                     "oracle_removed": oracle_removed, "oracle_added": oracle_added}
        if args.trace_exe:
            audit_row["added_second_join_matches"] = join_matches
        rows.append(audit_row)
    output = {
        "schema": "i07c-c2-fuzz-delta-audit-v1",
        "input_sha256": hashlib.sha256(args.input.read_bytes()).hexdigest(),
        "candidate_sha256": report["candidate_release_sha256"],
        "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
        "counts": dict(census), "rows": rows}
    if args.trace_exe:
        output["trace_sha256"] = gate.sha(args.trace_exe)
    args.output.write_text(json.dumps(output, indent=2) + "\n",
                           encoding="utf-8")
    print({key: census[key] for key in ("complete", "incomplete",
                                        "local_improvement_full_diagnostic")})


if __name__ == "__main__":
    main()
