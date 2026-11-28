#!/usr/bin/env python3
"""Audit all hard differences in a C2 fuzz report under D12.

The producer records every D/A-unmodeled hard difference. B rows equal the
complete oracle by construction. Historical exceptions require a frozen
source hash, profile, and exact output from the 64-member iteration.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate
from c2_d12_census import distance, new_false_positives

HERE = Path(__file__).resolve().parent


def historical_rows():
    iteration = json.loads((HERE / "c2-final-iteration-v15f.json").read_text(encoding="utf-8"))
    manifest = json.loads((HERE / "c2-family-members-v15f.json").read_text(encoding="utf-8"))
    member_ids = {row_id for family in manifest["families"].values()
                  for row_id in family["members"]}
    return {(iteration["rows"][row_id]["source_sha256"],
             iteration["rows"][row_id]["profile"]): (row_id, iteration["results"][row_id])
            for row_id in member_ids}


def compact_signature(compact):
    diagnostics = []
    for item in compact["diagnosticos"]:
        related = {relation["role"]: relation["span"] for relation in item["relations"]}
        diagnostics.append([item["code"], item["subcause"], item["primary"],
                            item["expected_type"], item["found_type"],
                            related.get("expected_type"), related.get("found_type")])
    return [compact["outcome"], diagnostics]


def validate_overrides(known):
    frozen = json.loads((HERE / "c2-oracle-overrides-v15f.json").read_text(encoding="utf-8"))
    for entry in frozen["entries"]:
        key = (entry["source_sha256"], entry["profile"])
        member = known.get(key)
        if member is None or member[0] != entry["historical_member"]:
            raise ValueError("oracle override is not a frozen historical member")
        candidate = member[1]["candidate"]["diagnosticos"]
        oracle = member[1]["oracle"]["diagnostics"]
        if not any([item["code"], item["subcause"], item["primary"]] ==
                   entry["candidate_diagnostic"] for item in candidate):
            raise ValueError("oracle override candidate diagnostic changed")
        if not any([item["code"], item["subcause"], item["primary"]] ==
                   entry["oracle_diagnostic"] for item in oracle):
            raise ValueError("oracle override oracle diagnostic changed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report-only", action="store_true")
    args = parser.parse_args()
    raw = args.input.read_bytes()
    report = json.loads(raw)
    known = historical_rows()
    validate_overrides(known)
    counts = collections.Counter()
    rows = []
    for row in report["hard_failures"]:
        if row["reason"] != "unclassified_difference":
            counts["hard_failure"] += 1
            rows.append({"seed": row["seed"], "status": "hard_failure", "reason": row["reason"]})
            continue
        source_hash = hashlib.sha256(row["source"].encode("ascii")).hexdigest()
        member = known.get((source_hash, report["profile"]))
        if member is not None and row["main"] == compact_signature(member[1]["main"]) and row[
            "candidate"] == compact_signature(member[1]["candidate"]
        ):
            # The original 64 were adjudicated separately by B. Their exact
            # output is checked by the frozen iteration/cross gate, not here.
            status = "historical_member"
            item = {"seed": row["seed"], "status": status, "member": member[0]}
        elif member is not None:
            status = "historical_output_changed"
            item = {"seed": row["seed"], "status": status, "member": member[0]}
        else:
            derived = gate.oracle.derive(row["source"])
            complete = not any(derived.get(key) for key in
                               ("error", "unmodeled", "no_fijado", "d_optional"))
            if complete:
                expected = gate.oracle_signature(derived)
                dm = distance(row["main"], expected)
                dc = distance(row["candidate"], expected)
                new_fp = new_false_positives(row["main"], row["candidate"], expected)
                status = ("improvement" if dc < dm and new_fp == 0
                          and row["main"][0] == row["candidate"][0] else "complete_defect")
                item = {"seed": row["seed"], "status": status, "distance_main": dm,
                        "distance_candidate": dc, "new_false_positives": new_fp}
            else:
                status = "conserve_main"
                item = {"seed": row["seed"], "status": status}
        counts[status] += 1
        rows.append(item)
    output = {"schema": "i07c-c2-d12-fuzz-gate-v1",
              "input_sha256": hashlib.sha256(raw).hexdigest(),
              "candidate_sha256": report["candidate_release_sha256"],
              "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
              "historical_members_sha256": gate.sha(HERE / "c2-family-members-v15f.json"),
              "oracle_overrides_sha256": gate.sha(HERE / "c2-oracle-overrides-v15f.json"),
              "complete": report["complete"], "counts": dict(counts), "rows": rows}
    args.output.write_text(json.dumps(output, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(dict(counts))
    if not args.report_only and (not report["complete"] or
                                  any(counts[name] for name in
                                      ("complete_defect", "conserve_main", "hard_failure",
                                       "historical_output_changed"))):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
