#!/usr/bin/env python3
"""Freeze reduced programs for the observed R6 regression families."""
import hashlib
import json
from pathlib import Path

from i07c_rd_drive_r6 import BASE, CANDIDATE, FLAGS
from i07c_rd_match103_r6 import inspect


CASES = {
    "match_nested_first": (
        "f0-0003-b-v1",
        "fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then (if true then 1 else q) else q);},None=>{value:true;}};}"),
    "t5_extra_constructor": (
        "f0-0002-b-v2",
        "fn Main(){var r=[];var s=[];let a=[(if true then (if 1 then r[0] else [1]) else s),r];}"),
    "t5_lost_assignment": (
        "f0-0002-b-v2",
        "fn Main(){var q=[];(if true then (if 1 then [[1]] else [[1]]) else q);q=[1];}"),
    "provenance_expected_type": (
        "f0-0002-b-v2",
        "fn Main(){let ar=[1];var q=[];var r=[];(if true then [q] else r);r=q;q=ar;}"),
}


def main():
    rows = []
    for name, (profile, source) in CASES.items():
        rows.append({"name": name, "profile": profile, "bytes": len(source.encode()),
                     "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                     "source": source, "main": inspect(BASE, source, profile, None),
                     "r6": inspect(CANDIDATE, source, profile, FLAGS)})
    doc = {"base_sha256": hashlib.sha256(Path(BASE).read_bytes()).hexdigest(),
           "r6_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
           "flags": FLAGS, "rows": rows}
    output = Path("docs/validation/i07c-rediseno/r6-regression-minima.json")
    output.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for row in rows:
        print(row["name"], row["bytes"],
              [(d["code"], d["primary"]) for d in row["main"]["diagnosticos"]],
              [(d["code"], d["primary"]) for d in row["r6"]["diagnosticos"]])


if __name__ == "__main__":
    main()
