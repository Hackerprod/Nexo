#!/usr/bin/env python3
"""Classify C2 differences by observable diagnostics and causal JSONL events.

The frozen member list audits the result; row IDs never decide a family.
"""

import argparse
import hashlib
import json
from pathlib import Path

from c2_e1303_family_probe import family_witness, read, published_sha

HERE = Path(__file__).resolve().parent
FROZEN = HERE / "c2-family-members-v15f.json"


def digest_ids(ids):
    return hashlib.sha256("\n".join(sorted(ids)).encode("utf-8")).hexdigest()


def relation(diagnostic, role):
    return next((item["span"] for item in diagnostic["relations"]
                 if item["role"] == role), None)


def contains(outer, inner):
    return outer[0] <= inner[0] and inner[1] <= outer[1]


def remove_one(haystack, needle):
    rest = list(haystack)
    if needle not in rest:
        return None
    rest.remove(needle)
    return rest


def only_delta(main, candidate):
    if main["outcome"] != candidate["outcome"]:
        return None
    m, c = main["diagnosticos"], candidate["diagnosticos"]
    for diagnostic in m:
        if remove_one(m, diagnostic) == c:
            return "main_extra", diagnostic
    for diagnostic in c:
        if remove_one(c, diagnostic) == m:
            return "candidate_extra", diagnostic
    return None


def witness_relation(main, candidate, events):
    m, c = main["diagnosticos"], candidate["diagnosticos"]
    if len(m) != len(c) or main["outcome"] != candidate["outcome"]:
        return None
    changed = [(left, right) for left, right in zip(m, c) if left != right]
    if len(changed) != 1:
        return None
    left, right = changed[0]
    old, new = relation(left, "expected_type"), relation(right, "expected_type")
    if left["code"] != "NX-E1303" or old is None or new is None or old == new:
        return None
    copy = dict(left)
    copy["relations"] = [dict(item, span=new) if item["role"] == "expected_type" else item
                         for item in left["relations"]]
    if copy != right:
        return None
    joins = [event for event in events if event.get("event") == "if_join_context"
             and not event.get("external_context")]
    for outer in joins:
        if outer.get("first_value") != new or not contains(outer["if_span"], right["primary"]):
            continue
        for inner in joins:
            if inner.get("first_value") != old or inner.get("second_value") != right["primary"]:
                continue
            if contains(outer["if_span"], inner["if_span"]) and inner["if_span"] != outer["if_span"]:
                return {"outer_ordinal": outer["owner_ordinal"], "inner_ordinal": inner["owner_ordinal"],
                        "primary": right["primary"], "old": old, "new": new}
    return None


def witness_assignment(main, candidate, events):
    m, c = main["diagnosticos"], candidate["diagnosticos"]
    if len(m) != len(c) or main["outcome"] != candidate["outcome"]:
        return None
    changed = [(left, right) for left, right in zip(m, c) if left != right]
    if len(changed) != 1:
        return None
    left, right = changed[0]
    if left["code"] != right["code"] or right["code"] != "NX-E0102":
        return None
    if left["subcause"] != right["subcause"] or right["subcause"] != "type_mismatch":
        return None
    if right["primary"][0] >= left["primary"][0]:
        return None
    chosen = relation(right, "expected_type")
    if chosen is None or chosen != relation(left, "expected_type"):
        return None
    recorded = any(event.get("event") == "diagnostic_recorded" and
                   event.get("code") == right["code"] and event.get("primary") == right["primary"]
                   for event in events)
    later = any(event.get("event") == "assignment" and event.get("value") == left["primary"]
                and event.get("chosen") == chosen for event in events)
    if recorded and later:
        return {"earlier_primary": right["primary"], "later_primary": left["primary"],
                "witness": chosen}
    return None


def constructor_witness(events, site, *, failed=None):
    return next((event for event in events if event.get("event") == "constructor_close"
                 and event.get("site") == site and event.get("element_concrete") is not None
                 and (failed is None or event.get("failed_constructor") == failed)), None)


def witness_compatible(main, candidate, events):
    delta = only_delta(main, candidate)
    if delta is None:
        return None
    direction, diagnostic = delta
    joins = [event for event in events if event.get("event") == "if_join_context"
             and not event.get("external_context")]
    recorded = [event for event in events if event.get("event") == "diagnostic_recorded"]
    if direction == "main_extra" and diagnostic["code"] == "NX-E0103":
        constructor = constructor_witness(events, diagnostic["primary"], failed=False)
        if constructor is None:
            return None
        if any(event.get("second_failed") and event.get("first_value")
               for event in joins):
            return {"kind": "resolved_constructor", "site": diagnostic["primary"],
                    "element": constructor["element_concrete"]}
    if direction == "candidate_extra" and diagnostic["code"] in ("NX-E0102", "NX-E1303"):
        if not any(event.get("code") == diagnostic["code"] and
                   event.get("primary") == diagnostic["primary"] for event in recorded):
            return None
        if diagnostic["code"] == "NX-E0102":
            constructors = [event for event in events if event.get("event") == "constructor_close"
                            and event.get("element_concrete") is not None
                            and event["site"][0] < diagnostic["primary"][0]]
            if not constructors or not any(event.get("second_failed") for event in joins):
                return None
            return {"kind": "later_assignment", "primary": diagnostic["primary"],
                    "constructor_sites": [event["site"] for event in constructors]}
        if any(event.get("second_value") == diagnostic["primary"] and
               event.get("first_value") and not event.get("external_context") for event in joins):
            return {"kind": "second_union", "primary": diagnostic["primary"]}
    return None


def witness_mixed(main, candidate, events):
    m, c = main["diagnosticos"], candidate["diagnosticos"]
    if main["outcome"] != candidate["outcome"] or len(m) != 2 or len(c) != 4:
        return None
    inference = next((d for d in m if d["code"] == "NX-E0103"), None)
    old_join = next((d for d in m if d["code"] == "NX-E1303"), None)
    if inference is None or old_join is None:
        return None
    constructor = constructor_witness(events, inference["primary"], failed=True)
    joins = [event for event in events if event.get("event") == "if_join_context"
             and not event.get("external_context")]
    shifted = next((d for d in c if d["code"] == "NX-E1303" and
                    any(event.get("second_value") == old_join["primary"] and
                        event.get("first_value") == d["primary"] for event in joins)), None)
    other_join = next((d for d in c if d["code"] == "NX-E1303" and d != shifted and
                       any(event.get("second_value") == d["primary"] for event in joins)), None)
    independent = [d for d in c if d["code"] == "NX-E0102" and
                   d["subcause"] == "type_mismatch"]
    recorded = [event for event in events if event.get("event") == "diagnostic_recorded"]
    if constructor and shifted and other_join and len(independent) == 2 and all(
        any(event.get("code") == d["code"] and event.get("primary") == d["primary"]
            for event in recorded) for d in c
    ):
        return {"constructor": inference["primary"], "shifted_join": [old_join["primary"], shifted["primary"]],
                "inner_join": other_join["primary"], "independent": [d["primary"] for d in independent]}
    return None


def classify(main, candidate, events):
    one = family_witness(main, candidate, events)
    if one is not None:
        return "single_e1303_per_union", one
    for family, check in (("outer_first_witness", witness_relation),
                          ("assignment_prefix", witness_assignment),
                          ("compatible_flattened_leaf", witness_compatible),
                          ("mixed_compatible_constraints", witness_mixed)):
        result = check(main, candidate, events)
        if result is not None:
            return family, result
    return None, None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iteration", type=Path, required=True)
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--freeze", action="store_true", help="write initial member manifest after complete classification")
    args = parser.parse_args()
    iteration, trace = read(args.iteration), read(args.trace)
    if not iteration["complete"] or set(iteration["rows"]) != set(trace["rows"]):
        raise ValueError("iteration and trace must cover the same frozen rows")
    if trace["frozen_sha256"] != hashlib.sha256(args.iteration.read_bytes()).hexdigest():
        raise ValueError("trace was not checked against this iteration")
    members, unresolved = {}, []
    for row_id, row in iteration["rows"].items():
        observed, traced = iteration["results"][row_id], trace["rows"][row_id]
        if row["source_sha256"] != traced["source_sha256"] or observed["candidate"] != traced["candidate"]:
            raise ValueError(f"trace mismatch: {row_id}")
        if observed["main"] == observed["candidate"]:
            continue
        family, witness = classify(observed["main"], observed["candidate"], traced["trace"])
        if family is None:
            unresolved.append(row_id)
        else:
            members[row_id] = {"family": family, "witness": witness,
                               "source_sha256": row["source_sha256"]}
    by_family = {}
    for family in sorted({item["family"] for item in members.values()}):
        ids = [row_id for row_id, item in members.items() if item["family"] == family]
        by_family[family] = {"count": len(ids), "members_sha256": digest_ids(ids), "members": sorted(ids)}
    report = {"schema": "i07c-c2-family-check-v1", "iteration_sha256": published_sha(args.iteration),
              "trace_sha256": published_sha(args.trace), "members": members,
              "families": by_family, "unresolved": sorted(unresolved)}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    if args.freeze:
        if unresolved or len(members) != 64:
            raise ValueError("cannot freeze incomplete C2 classification")
        FROZEN.write_text(json.dumps({"schema": "i07c-c2-family-members-v1", "families": by_family},
                                     ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    elif FROZEN.exists():
        frozen = json.loads(FROZEN.read_text(encoding="utf-8"))
        if by_family != frozen["families"] or unresolved:
            raise ValueError("causal membership differs from frozen family manifest")
    print({family: data["count"] for family, data in by_family.items()},
          "unresolved", len(unresolved))
    if unresolved:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
