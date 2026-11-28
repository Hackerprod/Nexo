#!/usr/bin/env python3
"""Check machine-readable causal trace invariants without granting ADR PASS."""

import gzip
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "c2-r25-trace-120.json.gz"
OUTPUT = HERE / "c2-r25-trace-invariants.json"


def main():
    report = json.loads(gzip.decompress(SOURCE.read_bytes()))
    counts = {"rows": 0, "events": 0, "equalities": 0, "rejected": 0,
              "rejected_published": 0, "missing_ast_ordinal": 0,
              "selected_published": 0, "selected_fallback": 0,
              "recorded_final_diagnostics": 0, "unrecorded_final_diagnostics": 0}
    for row_id, row in report["rows"].items():
        events = row["trace"]
        counts["rows"] += 1
        counts["events"] += len(events)
        entered = {event["equality_id"]: event for event in events if event["event"] == "enter"}
        finished = {event["equality_id"]: event for event in events if event["event"] == "finish"}
        published = {event["equality_id"] for event in events if event["event"] == "publish"}
        if entered.keys() != finished.keys():
            raise ValueError(f"{row_id}: unmatched equality events")
        for equality_id, finish in finished.items():
            if entered[equality_id]["source_ordinal"] != finish["source_ordinal"]:
                raise ValueError(f"{row_id}: unstable AST ordinal at {equality_id}")
            if finish["source_ordinal"] is None:
                counts["missing_ast_ordinal"] += 1
            if finish["state"] == "rejected":
                counts["rejected"] += 1
                if equality_id in published:
                    counts["rejected_published"] += 1
            counts["equalities"] += 1
        for event in events:
            if event["event"] != "select":
                continue
            if event["witness_provenance"] == "published_origin_fact":
                counts["selected_published"] += 1
                if not any(fact["equality_id"] == event["witness_equality_id"]
                           for fact in event["candidate_facts"]):
                    raise ValueError(f"{row_id}: chosen witness absent from facts")
            else:
                counts["selected_fallback"] += 1
                if event["chosen"] != event["first_operand"]:
                    raise ValueError(f"{row_id}: fallback changed first operand")
        recorded = {(event["code"], tuple(event["primary"])) for event in events
                    if event["event"] == "diagnostic_recorded" and event["primary"]}
        for diagnostic in row["candidate"]["diagnosticos"]:
            key = (diagnostic["code"], tuple(diagnostic["primary"]))
            if key in recorded:
                counts["recorded_final_diagnostics"] += 1
            else:
                counts["unrecorded_final_diagnostics"] += 1
    if counts["rejected_published"] or counts["missing_ast_ordinal"]:
        raise ValueError(counts)
    OUTPUT.write_text(json.dumps({"schema": "c2-r25-trace-invariants-v1", "counts": counts},
                                 indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
