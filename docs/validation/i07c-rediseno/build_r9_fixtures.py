#!/usr/bin/env python3
"""Build the reviewed r9 witness table from frozen, preexisting source fixtures."""

import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def read(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def write(name, value):
    (HERE / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


f2 = read("r8-f2-ajeno4.json")
f2["schema"] = "i07c-r9-f2-ajeno4-v2"
row = next(x for x in f2["rows"] if x["id"] == "c1-13262")
row["expected_code_subcause_primary_types_relation"][0][-1] = [90, 93]
row["expected_authority"] = "F0-0002-lagunas.md, Origen de expected_type, primer EqualityId compatible: r=[3] [90,93); diferencia de procedencia I frente a main"
row["declared_difference_from_main"] = {"field": "relations.expected_type", "main": [142, 143], "normative": [90, 93]}
write("r8-f2-ajeno4.json", f2)

match = read("r8-match-main.json")["rows"]
m1 = read("r8-m1.json")["rows"]
frozen = read("r7-frozen-batteries.json")["probes"]
selected = [
    ("M1-simple", next(x for x in m1 if x["id"] == "M1-simple"), "m1"),
    ("M1-nested", next(x for x in m1 if x["id"] == "M1-nested"), "m1"),
]
selected += [(f"match-index-{i}", match[i], "match") for i in (13, 24, 32, 53)]
selected += [(name, next(x for x in frozen if x["id"] == name), "frozen") for name in ("W1_atomic_compound", "M2_partial_witness_chain")]
selected += [("match-4363", next(x for x in match if "4363" in x["id"]), "match")]
extra = {
    "W1_atomic_compound": [
        ("NX-E0102", "type_mismatch", [99, 100], "Bool", "I64", None, [99, 100]),
        ("NX-E1303", None, [147, 148], "Array<Array<Array<Array<I64>>>>", "Array<Array<Array<Array<Array<I64>>>>>", [137, 141], [147, 148]),
        ("NX-E1303", None, [199, 200], "Array<Array<Array<Array<I64>>>>", "Array<Array<Array<I64>>>", [85, 149], [199, 200]),
        ("NX-E1303", None, [209, 212], "Array<Array<Array<Array<I64>>>>", "Array<Array<I64>>", [74, 202], [209, 212]),
        ("NX-E0102", "type_mismatch", [216, 219], "Array<Array<Array<Array<Array<I64>>>>>", "Array<Array<I64>>", [170, 171], [216, 219]),
    ],
    "M2_partial_witness_chain": [
        ("NX-E0102", "type_mismatch", [81, 82], "Bool", "I64", None, [81, 82]),
        ("NX-E1303", None, [131, 132], "Array<Array<Bool>>", "Array<Array<Array<Bool>>>", [121, 125], [131, 132]),
        ("NX-E1303", None, [155, 158], "Array<Array<Bool>>", "Array<Array<I64>>", [56, 148], [155, 158]),
    ],
}
out = []
for name, src, kind in selected:
    assert hashlib.sha256(src["source"].encode()).hexdigest() == src["source_sha256"]
    item = {"id": name, "fixture_id": src["id"], "profile": src["profile"], "source": src["source"], "source_sha256": src["source_sha256"]}
    if kind == "frozen":
        item["expected_normative_code_primary"] = src["expected_code_primary"]
        item["expected_main"] = None
        item["expected_normative"] = {
            "outcome": "rejected",
            "diagnosticos": [
                {"code": code, "subcause": subcause, "primary": primary,
                 "expected_type": expected, "found_type": found,
                 "relations": ([{"role": "expected_type", "span": er}] if er else [])
                              + ([{"role": "found_type", "span": fr}] if fr else [])}
                for code, subcause, primary, expected, found, er, fr in extra[name]
            ],
        }
    else:
        item["expected_main"] = src["expected_main"]
        item["expected_normative"] = src["expected_main"]
    out.append(item)
write("r9-witness-discriminator.json", {"schema": "i07c-r9-witness-v1", "index_base": 0, "base_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548", "rows": out})
print("f2 relation", row["expected_code_subcause_primary_types_relation"][0][-1], "witnesses", len(out))
