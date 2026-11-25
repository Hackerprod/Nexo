#!/usr/bin/env python3
"""List every modeled R6 regression and group it by changed diagnostic fields."""
import hashlib
import json
from pathlib import Path

import i07c_rd_sweep as sweep
from i07c_rd_drive_r6 import CORPORA


def selected(value, width):
    return [item[:width] for item in value]


def main():
    base = "main-44bef37452e4"
    candidate = "proto-66c2cf310e7a-r6"
    records = []
    distribution = {}
    for corpus in CORPORA:
        rows = sweep.data(corpus)
        expected = sweep.expected(corpus, rows)
        original, current = sweep.load(base, corpus), sweep.load(candidate, corpus)
        counts = {"core_a": 0, "full_a": 0, "full_a_types": 0,
                  "full_a_relation_only": 0}
        for index, oracle in enumerate(expected):
            if oracle is None:
                continue
            b, c = original[index], current[index]
            short_full = corpus.split("@", 1)[0] in ("c2", "c2-nueva", "c2-nueva2") and isinstance(rows[index], dict) and rows[index].get("form") in (
                "annot", "arg", "ret", "option", "result", "takeempty", "matchexpr")
            core_a = selected(b, 4) == selected(oracle, 4) and selected(c, 4) != selected(oracle, 4)
            full_width = 6 if short_full else 7
            full_a = selected(b, full_width) == selected(oracle, full_width) and selected(c, full_width) != selected(oracle, full_width)
            if not core_a and not full_a:
                continue
            if core_a:
                kind = "core"
            elif selected(c, 6) != selected(oracle, 6):
                kind = "types"
            else:
                kind = "relation_only"
            counts["core_a"] += int(core_a)
            counts["full_a"] += int(full_a)
            if full_a and kind == "types":
                counts["full_a_types"] += 1
            if full_a and kind == "relation_only":
                counts["full_a_relation_only"] += 1
            source = sweep.source(rows[index])
            records.append({"corpus": corpus, "index": index, "kind": kind,
                            "core_a": core_a, "full_a": full_a,
                            "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                            "source": source if kind == "core" else None,
                            "oracle": oracle if kind == "core" else None,
                            "main": b if kind == "core" else None,
                            "r6": c if kind == "core" else None})
        distribution[corpus] = counts
    document = {"base_tag": base, "candidate_tag": candidate,
                "distribution": distribution, "records": records}
    dest = Path("docs/validation/i07c-rediseno/r6-regression-audit.json")
    dest.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"records": len(records),
                      "totals": {kind: sum(row[kind] for row in distribution.values())
                                 for kind in ("core_a", "full_a", "full_a_types",
                                              "full_a_relation_only")}}))


if __name__ == "__main__":
    main()
