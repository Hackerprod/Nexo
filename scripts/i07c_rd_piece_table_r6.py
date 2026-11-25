#!/usr/bin/env python3
"""A/B/C/D by R6 design piece on independently specified directed cases."""
from collections import Counter
import hashlib
import json
from pathlib import Path

from i07c_rd_drive_r6 import BASE, CANDIDATE, FLAGS
from i07c_rd_match103_r6 import inspect


PIECES = {
    "EqualityRecord": [13, 14, 15, 23, 24],
    "ReadContext": [17, 18, 19, 22, 25, 28],
    "ResultStatus/JoinRecord/T5": [1, 2, 3, 4, 5, 6, 7, 8, 9, 20, 21, 26, 30, 31, 32],
    "ConstructorDuty": [10, 11, 12, 14, 15, 27],
    "Waiting/Suspended": [16, 17, 18, 19, 33, 34, 35],
}
EXTRA = {
    "ReadContext": ["RC1_read_context_variable", "RC2_read_context_committed", "RC3_read_context_no_retroactive"],
    "ResultStatus/JoinRecord/T5": ["W1a_without_later_assignment", "M2_partial_witness_chain",
                                   "RS1_condition_contaminated", "RS2_independent_child_errors", "RS3_partial_in_array"],
    "ConstructorDuty": ["CD1_independent_constructor", "CD2_independent_error_and_duties"],
    "Waiting/Suspended": ["SU1_suspended_order", "SU2_suspended_chain"],
}


def category(base, candidate, oracle):
    if base == oracle and candidate != oracle:
        return "A"
    if base != oracle and candidate == oracle:
        return "B"
    if base == candidate == oracle:
        return "OK"
    return "C" if base == candidate else "D"


def main():
    directed = json.loads(Path("docs/validation/i07c-rediseno/r6-r4-directed.json").read_text(encoding="utf-8"))
    source_rows = {r["name"]: r for r in directed["rows"]}
    pieces = {}
    for piece, indices in PIECES.items():
        names = [f"B3_{i:02}" for i in indices] + EXTRA.get(piece, [])
        results = []
        for name in names:
            row = source_rows[name]
            profile, source = "f0-0002-b-v2", row["source"]
            base = inspect(BASE, source, profile, None)
            canonical = lambda values: sorted(values, key=lambda d: (d[1], d[2], d[0]))
            base_sig = canonical([[d["code"], *d["primary"]] for d in base["diagnosticos"]])
            candidate_sig = canonical(row["actual"])
            oracle = canonical(row["expected"])
            results.append({"name": name, "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                            "category": category(base_sig, candidate_sig, oracle),
                            "base": base_sig, "candidate": candidate_sig, "oracle": oracle})
        pieces[piece] = {"counts": dict(Counter(r["category"] for r in results)), "rows": results}
    doc = {"scope": "B3 35, W1a, M2, B r4 new10; overlaps allowed across pieces",
           "base_sha256": hashlib.sha256(Path(BASE).read_bytes()).hexdigest(),
           "candidate_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
           "flags": FLAGS, "pieces": pieces}
    output = Path("docs/validation/i07c-rediseno/r6-piece-directed.json")
    output.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for name, data in pieces.items():
        print(name, len(data["rows"]), data["counts"])


if __name__ == "__main__":
    main()
