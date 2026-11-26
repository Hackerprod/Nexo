#!/usr/bin/env python3
"""Verify every frozen I07c R6 source corpus before a gate."""

import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
FILE = {
    "c1": "c1.json", "c2": "c2.json", "b": "b.json", "perm": "perm.json",
    "t": "t.json", "dirigido5": "dirigido5.json",
    "c1-nueva": "c1-nueva.json", "c2-nueva": "c2-nueva.json",
    "c1-nueva2": "c1-nueva2.json", "c2-nueva2": "c2-nueva2.json",
    "s7021993": "b-7021993.json", "s4417061": "b-4417061.json", "s9908255": "b-9908255.json",
    "b-projections-2026112401": "b-projections-2026112401.json",
    "b-projections-t5-2026112402": "b-projections-t5-2026112402.json",
    "b-projections-2026112403": "b-projections-2026112403.json",
    "owner-projections-771412": "owner-projections-771412.json",
}


def main():
    entries = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    assert len(entries) == 20
    for entry in entries:
        name = entry["id"].split("@")[0]
        path = HERE / FILE[name]
        data = path.read_bytes()
        sha = hashlib.sha256(data).hexdigest()
        assert sha == entry["sha256"], (entry["id"], sha, entry["sha256"])
        rows = json.loads(data)
        if isinstance(rows, dict):
            rows = rows.get("progs", rows.get("rows", rows))
        assert len(rows) == entry["rows"], (entry["id"], len(rows), entry["rows"])
        print(entry["id"], entry["profile"], len(rows), sha)
    assert sum(e["rows"] for e in entries) == 151188


if __name__ == "__main__":
    main()
