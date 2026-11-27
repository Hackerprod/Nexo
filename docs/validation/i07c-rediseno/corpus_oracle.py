#!/usr/bin/env python3
"""Derive type expectations per frozen corpus row without invoking Nexo.

Run one corpus/profile at a time:
  python -B docs/validation/i07c-rediseno/corpus_oracle.py c1 OUTPUT.jsonl
The output contains a record for every row, including explicitly unmodeled
rows. Its SHA must be recorded by the gate before comparing a candidate.
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
import i07c_rd_oracle as oracle  # noqa: E402
from verify_corpora import FILE  # noqa: E402


def source(row):
    if isinstance(row, str):
        return row
    for key in ("wrapped", "w", "flat", "source"):
        if key in row:
            return row[key]
    raise ValueError("corpus row has no source")


def main():
    corpus_id, output = sys.argv[1:3]
    entries = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    entry = next(x for x in entries if x["id"] == corpus_id)
    blob = (HERE / FILE[corpus_id.split("@")[0]]).read_bytes()
    assert hashlib.sha256(blob).hexdigest() == entry["sha256"]
    rows = json.loads(blob)
    if isinstance(rows, dict):
        rows = rows["progs"]
    assert len(rows) == entry["rows"]
    counts = {"modeled_full": 0, "partial_or_unmodeled": 0}
    path = Path(output)
    with path.open("w", encoding="utf-8", newline="\n") as stream:
        for index, row in enumerate(rows):
            src = source(row)
            result = oracle.derive(src)
            full = not result.get("error") and not result.get("unmodeled") and not result.get("no_fijado") and not result.get("d_optional")
            counts["modeled_full" if full else "partial_or_unmodeled"] += 1
            record = {"corpus": corpus_id, "index": index, "profile": entry["profile"],
                      "source_sha256": hashlib.sha256(src.encode("utf-8")).hexdigest(),
                      "modeled_full": full, "oracle": result}
            stream.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(json.dumps({"corpus": corpus_id, "rows": len(rows), "counts": counts,
                      "output_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}, sort_keys=True))


if __name__ == "__main__":
    main()
