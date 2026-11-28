#!/usr/bin/env python3
"""Conservative C2 triage of the 120 D8 rows; never grants a family exception.

An oracle C absent from the candidate is a blocking candidate defect even if
another part of that source is unmodeled. The two manually derived relation
defects are kept separate from mechanically identified missing C sites.
"""

import argparse
import hashlib
import json
from pathlib import Path

import c2_delta_audit

HERE = Path(__file__).resolve().parent
ITERATION = HERE / "c2-d8-iteration-pb.json"
AUDIT = HERE / "c2-d8-delta-audit.json"

MANUAL = {
    "b-projections-2026112401:233": {
        "primary": [168, 169], "expected_relation": [81, 82],
        "clause": "F0-0002 §§4.1.2.4, 4.1.2.6",
    },
    "b-projections-2026112401:1995": {
        "primary": [159, 160], "expected_relation": [91, 95],
        "clause": "F0-0002 §§4.1.2.4–6",
    },
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def actual_relation(item):
    return c2_delta_audit.related(item, "expected_type")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    iteration = json.loads(ITERATION.read_text(encoding="utf-8"))
    audit = json.loads(AUDIT.read_text(encoding="utf-8"))
    if not iteration["complete"] or set(iteration["rows"]) != set(audit["rows"]):
        raise ValueError("the frozen 120-row universe changed")
    if len(iteration["rows"]) != 120 or audit["iteration_sha256"] != sha(ITERATION.read_bytes()):
        raise ValueError("audit does not match the frozen iteration")
    rows = {}
    for row_id, definition in iteration["rows"].items():
        if sha(definition["source"].encode("utf-8")) != definition["source_sha256"]:
            raise ValueError(f"source SHA mismatch: {row_id}")
        observed = iteration["results"][row_id]
        candidate = observed["candidate"]["diagnosticos"]
        candidate_by_key = {c2_delta_audit.key(item): item for item in candidate}
        if len(candidate_by_key) != len(candidate):
            raise ValueError(f"duplicate candidate site: {row_id}")
        required = [item for item in observed["oracle"]["diagnostics"]
                    if item.get("cls") == "C"]
        missing = [item for item in required
                   if c2_delta_audit.key(item) not in candidate_by_key]
        manual = MANUAL.get(row_id)
        if manual:
            key = ("NX-E0102", "type_mismatch", tuple(manual["primary"]))
            actual = candidate_by_key.get(key)
            if actual is None:
                raise ValueError(f"manual diagnostic disappeared: {row_id}")
            manual = {**manual, "candidate_relation": actual_relation(actual)}
            if manual["candidate_relation"] == manual["expected_relation"]:
                raise ValueError(f"manual defect no longer reproduces: {row_id}")
        rows[row_id] = {
            "source_sha256": definition["source_sha256"],
            "profile": definition["profile"],
            "delta_kinds": [delta["kind"] for delta in audit["rows"][row_id]["deltas"]],
            "missing_required_c": [
                {"code": item["code"], "subcause": item["subcause"],
                 "primary": item["primary"], "clause": item["clause"]}
                for item in missing
            ],
            "manual_relation_defect": manual,
            "triage": "candidate_defect" if missing or manual else "requires_causal_derivation",
        }
    report = {
        "schema": "i07c-c2-d8-blockers-v1",
        "iteration_sha256": sha(ITERATION.read_bytes()),
        "audit_sha256": sha(AUDIT.read_bytes()),
        "counts": {
            "rows": len(rows),
            "candidate_defect_rows": sum(row["triage"] == "candidate_defect"
                                         for row in rows.values()),
            "missing_required_c_rows": sum(bool(row["missing_required_c"])
                                           for row in rows.values()),
            "missing_required_c_sites": sum(len(row["missing_required_c"])
                                            for row in rows.values()),
            "manual_relation_defect_rows": len(MANUAL),
            "requires_causal_derivation": sum(row["triage"] == "requires_causal_derivation"
                                               for row in rows.values()),
        },
        "rows": rows,
    }
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n",
                           encoding="utf-8")
    print(report["counts"])


if __name__ == "__main__":
    main()
