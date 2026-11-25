#!/usr/bin/env python3
"""Freeze exact modeled A rows from the R5 differential."""

import argparse
import hashlib
import json
from pathlib import Path

import i07c_rd_sweep as sweep


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus", nargs="+")
    args = parser.parse_args()
    binary = Path("C:/nexo-work/tmp/r5-statement-82729b1f.exe")
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if digest != "82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb":
        raise RuntimeError("R5 binary identity changed")
    rows = []
    for name in args.corpus:
        sources = sweep.data(name)
        expected = sweep.expected(name, sources)
        base = sweep.load("main-44bef37452e4", name)
        candidate = sweep.load("proto-82729b1f2b6c-r5owner", name)
        for index in sorted(set(base) & set(candidate)):
            oracle = expected[index]
            if oracle is None:
                continue
            base_core = [r[:4] for r in base[index]]
            candidate_core = [r[:4] for r in candidate[index]]
            oracle_core = [r[:4] for r in oracle]
            if base_core == oracle_core and candidate_core != oracle_core:
                level = "core"
            elif base[index] == oracle and candidate[index] != oracle:
                level = "full"
            else:
                continue
            rows.append({"corpus": name, "index": index, "level": level,
                         "source": sweep.source(sources[index]), "oracle": oracle,
                         "main": base[index], "candidate": candidate[index]})
    dest = Path("docs/validation/i07c-rediseno/r5-modeled-regressions.json")
    dest.write_text(json.dumps({"candidate_sha256": digest, "corpora": args.corpus,
                                "rows": rows},
                               ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"rows": len(rows), "core": sum(r["level"] == "core" for r in rows),
                      "full_only": sum(r["level"] == "full" for r in rows)}))


if __name__ == "__main__":
    main()
