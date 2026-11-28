#!/usr/bin/env python3
"""Audit individual C2 deltas without granting an ADR family exception.

This tool deliberately leaves partial-oracle cases unresolved. A mechanical
delta is evidence for a review, not a substitute for the manual ADR derivation.
"""

import argparse
import collections
import hashlib
import json
from pathlib import Path


def digest(value):
    return hashlib.sha256(value).hexdigest()


def key(item):
    return item["code"], item["subcause"], tuple(item["primary"])


def related(item, role):
    return next((r["span"] for r in item["relations"] if r["role"] == role), None)


def audit(row):
    observed = row["observed"]
    main = observed["main"]["diagnosticos"]
    candidate = observed["candidate"]["diagnosticos"]
    expected = observed["oracle"]["diagnostics"]
    complete = not observed["oracle_modeling"]
    main_by_key = {key(item): item for item in main}
    candidate_by_key = {key(item): item for item in candidate}
    expected_by_key = {key(item): item for item in expected}
    deltas = []

    for identity, item in main_by_key.items():
        if identity not in candidate_by_key:
            verdict = ("candidate_violation" if identity in expected_by_key else "main_violation") if complete else "unresolved"
            deltas.append({"kind": "removed_diagnostic", "identity": identity,
                           "main": item, "candidate": None, "oracle": expected_by_key.get(identity),
                           "local_verdict": verdict})
            continue
        other = candidate_by_key[identity]
        for role in ("expected_type", "found_type"):
            before, after = related(item, role), related(other, role)
            if before == after:
                continue
            oracle_item = expected_by_key.get(identity)
            oracle_span = oracle_item["rel_expected" if role == "expected_type" else "rel_found"] if oracle_item else None
            if oracle_item is None and not complete:
                verdict = "unresolved"
            elif before == oracle_span and after != oracle_span:
                verdict = "candidate_violation"
            elif after == oracle_span and before != oracle_span:
                verdict = "main_violation"
            else:
                verdict = "unresolved"
            deltas.append({"kind": "relation", "identity": identity, "role": role,
                           "main": before, "candidate": after, "oracle": oracle_span,
                           "local_verdict": verdict})
        for field in ("expected_type", "found_type"):
            if item[field] != other[field]:
                deltas.append({"kind": "data", "identity": identity, "field": field,
                               "main": item[field], "candidate": other[field],
                               "oracle": expected_by_key.get(identity, {}).get(field),
                               "local_verdict": "unresolved"})

    for identity, item in candidate_by_key.items():
        if identity not in main_by_key:
            verdict = ("main_violation" if identity in expected_by_key else "candidate_violation") if complete else "unresolved"
            deltas.append({"kind": "added_diagnostic", "identity": identity,
                           "main": None, "candidate": item, "oracle": expected_by_key.get(identity),
                           "local_verdict": verdict})
    assert deltas, "the input row has no diagnostic delta"
    if any(item["local_verdict"] == "candidate_violation" for item in deltas):
        verdict = "candidate_defect"
    elif any(item["local_verdict"] == "unresolved" for item in deltas):
        verdict = "unresolved"
    else:
        verdict = "main_violation_delta"
    conflicts = []
    for identity, oracle_item in expected_by_key.items():
        if oracle_item.get("cls") != "C":
            continue
        item = candidate_by_key.get(identity)
        if item is None:
            conflicts.append({"identity": identity, "field": "required_c", "oracle": oracle_item})
            continue
        for field in ("expected_type", "found_type"):
            if (item[field] or "") != (oracle_item[field] or ""):
                conflicts.append({"identity": identity, "field": field,
                                  "candidate": item[field], "oracle": oracle_item[field]})
        for role, field in (("expected_type", "rel_expected"), ("found_type", "rel_found")):
            if related(item, role) != oracle_item[field]:
                conflicts.append({"identity": identity, "field": field,
                                  "candidate": related(item, role), "oracle": oracle_item[field]})
    return verdict, deltas, conflicts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iteration", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    iteration = json.loads(args.iteration.read_text(encoding="utf-8"))
    if not iteration.get("complete") or len(iteration["rows"]) != 120:
        parser.error("expected the complete 120-row iteration report")
    results = {}
    for row_id, definition in iteration["rows"].items():
        assert digest(definition["source"].encode("utf-8")) == definition["source_sha256"], row_id
        observed = iteration["results"][row_id]
        verdict, deltas, conflicts = audit({"observed": observed})
        results[row_id] = {"source": definition["source"],
                           "source_sha256": definition["source_sha256"],
                           "profile": definition["profile"],
                           "baseline_class": definition["baseline_class"],
                           "verdict": verdict, "deltas": deltas,
                           "candidate_oracle_conflicts": conflicts}
    counts = dict(sorted(collections.Counter(x["verdict"] for x in results.values()).items()))
    report = {"schema": "i07c-c2-d8-delta-audit-v1", "iteration_sha256": digest(args.iteration.read_bytes()),
              "main_sha256": iteration["main_sha256"], "candidate_sha256": iteration["candidate_sha256"],
              "counts": counts,
              "rows_with_candidate_oracle_conflicts": sum(bool(x["candidate_oracle_conflicts"]) for x in results.values()),
              "rows": results}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(counts)


if __name__ == "__main__":
    main()
