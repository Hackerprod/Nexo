#!/usr/bin/env python3
"""Check complete per-function type-analysis pass records from a gate run.

Usage: python -B scripts/i07c_rd_pass_meter.py EXPECTED.jsonl ACTUAL.jsonl
Both files contain source_sha256, profile, mode, build, function_ordinal.
ACTUAL additionally contains passes. The harness creates EXPECTED before runs.
"""

import collections
import json
import sys
from pathlib import Path

KEY = ("source_sha256", "profile", "mode", "build", "function_ordinal")


def read(path):
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def key(row):
    return tuple(row[name] for name in KEY)


def main():
    expected, actual = map(read, sys.argv[1:3])
    a, b = collections.Counter(map(key, expected)), collections.Counter(map(key, actual))
    if a != b or any(count != 1 for count in a.values()):
        raise SystemExit(f"missing/duplicate pass records: missing={sum((a-b).values())} extra={sum((b-a).values())}")
    histogram = collections.Counter()
    worst = None
    for row in actual:
        passes = row["passes"]
        if not isinstance(passes, int) or passes < 1:
            raise SystemExit(f"invalid passes: {row}")
        histogram[passes] += 1
        if worst is None or passes > worst["passes"]:
            worst = row
    report = {"records": len(actual), "max_passes": worst["passes"] if worst else None,
              "worst": worst, "histogram": dict(sorted(histogram.items()))}
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    if not actual or worst["passes"] > 3:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
