#!/usr/bin/env python3
"""Adjudicate the 73 frozen R5 rows against the completed R6 sweep."""

from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess

import i07c_rd_sweep as sweep

FROZEN_COMMIT = "f4c47df"
FROZEN_PATH = "docs/validation/i07c-rediseno/r5-modeled-regressions.json"
BASE = "main-44bef37452e4"
CANDIDATE = "proto-66c2cf310e7a-r6"
DEST = Path("docs/validation/i07c-rediseno/r6-modeladas-73.json")


def signature(value, level, short_full):
    if level == "core":
        return [item[:4] for item in value]
    if short_full:
        return [item[:6] for item in value]
    return value


def main():
    original = subprocess.check_output(
        ["git", "show", f"{FROZEN_COMMIT}:{FROZEN_PATH}"]
    )
    frozen = json.loads(original)
    assert len(frozen["rows"]) == 73
    cache = {}
    results = []
    for item in frozen["rows"]:
        corpus = item["corpus"]
        if corpus not in cache:
            sources = sweep.data(corpus)
            cache[corpus] = (
                sources,
                sweep.load(BASE, corpus),
                sweep.load(CANDIDATE, corpus),
                sweep.expected(corpus, sources),
            )
        sources, base, candidate, expected = cache[corpus]
        index = item["index"]
        row = sources[index]
        assert sweep.source(row) == item["source"], (corpus, index)
        assert expected[index] is not None, (corpus, index)
        short_full = (
            corpus.split("@", 1)[0] in ("c2", "c2-nueva", "c2-nueva2")
            and isinstance(row, dict)
            and row.get("form") in (
                "annot", "arg", "ret", "option", "result", "takeempty", "matchexpr"
            )
        )
        level = item["level"]
        b = signature(base[index], level, short_full)
        c = signature(candidate[index], level, short_full)
        o = signature(expected[index], level, short_full)
        category = (
            "A" if b == o and c != o else
            "B" if b != o and c == o else
            "OK" if b == o else
            "C" if b == c else "D"
        )
        results.append({
            "corpus": corpus,
            "index": index,
            "level": level,
            "source_sha256": hashlib.sha256(item["source"].encode()).hexdigest(),
            "category": category,
            "oracle": o,
            "main": b,
            "r6": c,
        })
    counts = dict(Counter(item["category"] for item in results))
    document = {
        "frozen_commit": FROZEN_COMMIT,
        "frozen_path": FROZEN_PATH,
        "frozen_sha256": hashlib.sha256(original).hexdigest(),
        "base_tag": BASE,
        "candidate_tag": CANDIDATE,
        "counts": counts,
        "rows": results,
    }
    DEST.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"rows": len(results), "counts": counts}))


if __name__ == "__main__":
    main()
