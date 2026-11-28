#!/usr/bin/env python3
"""Inspect the provisional one-E1303-per-if family in an iteration and JSONL trace.

This is an iteration probe, not the frozen C2 cross gate. It never accepts a
row by corpus name, row number, or source span.
"""

import argparse
import gzip
import hashlib
import json
from pathlib import Path


def read(path):
    data = gzip.open(path, "rt", encoding="utf-8") if path.suffix == ".gz" else path.open(encoding="utf-8")
    with data:
        return json.load(data)


def contains(outer, inner):
    return outer[0] <= inner[0] and inner[1] <= outer[1]


def expected_relation(diagnostic):
    return next((relation["span"] for relation in diagnostic["relations"]
                 if relation["role"] == "expected_type"), None)


def family_witness(main, candidate, events):
    if main["outcome"] != candidate["outcome"]:
        return None
    remaining = list(main["diagnosticos"])
    for diagnostic in candidate["diagnosticos"]:
        if diagnostic not in remaining:
            return None
        remaining.remove(diagnostic)
    if len(remaining) != 1 or remaining[0]["code"] != "NX-E1303":
        return None
    omitted = remaining[0]
    earlier = [diagnostic for diagnostic in candidate["diagnosticos"]
               if diagnostic["code"] == "NX-E1303"
               and diagnostic["primary"][0] < omitted["primary"][0]
               and expected_relation(diagnostic) == expected_relation(omitted)]
    contexts = []
    for event in events:
        if event.get("event") != "if_join_context" or event.get("external_context"):
            continue
        if not contains(event["if_span"], omitted["primary"]):
            continue
        if event["first_block"] != event["first_value"]:
            continue
        if event["second_block"] != event["second_value"]:
            continue
        prior = next((diagnostic for diagnostic in earlier
                      if contains(event["if_span"], diagnostic["primary"])), None)
        if prior is not None:
            contexts.append((event["if_span"][1] - event["if_span"][0], event, prior))
    if not contexts:
        return None
    if any(event.get("event") == "finish" and event.get("state") == "rejected"
           and event.get("primary") == omitted["primary"] for event in events):
        return None
    _, context, prior = min(contexts, key=lambda item: item[0])
    return {"owner_ordinal": context["owner_ordinal"],
            "omitted": omitted, "prior": prior,
            "clause": "F0-0002 §4.1.2.4; N(d)"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iteration", type=Path, required=True)
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    iteration, trace = read(args.iteration), read(args.trace)
    if not iteration["complete"] or set(iteration["rows"]) != set(trace["rows"]):
        raise ValueError("iteration or trace does not cover exactly the frozen rows")
    members = {}
    unresolved = []
    for row_id, row in iteration["rows"].items():
        observed = iteration["results"][row_id]
        traced = trace["rows"][row_id]
        if row["source_sha256"] != traced["source_sha256"] or observed["candidate"] != traced["candidate"]:
            raise ValueError(f"trace mismatch: {row_id}")
        if observed["main"] == observed["candidate"]:
            continue
        witness = family_witness(observed["main"], observed["candidate"], traced["trace"])
        if witness is None:
            unresolved.append(row_id)
        else:
            members[row_id] = witness
    ids = sorted(members)
    report = {"schema": "i07c-c2-e1303-family-probe-v1",
              "iteration_sha256": hashlib.sha256(args.iteration.read_bytes()).hexdigest(),
              "trace_sha256": hashlib.sha256(args.trace.read_bytes()).hexdigest(),
              "members_sha256": hashlib.sha256("\n".join(ids).encode()).hexdigest(),
              "members": members, "unresolved": sorted(unresolved)}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(len(members), "family members;", len(unresolved), "unresolved; SHA", report["members_sha256"])


if __name__ == "__main__":
    main()
