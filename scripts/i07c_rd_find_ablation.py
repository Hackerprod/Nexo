#!/usr/bin/env python3
"""Find a frozen source that requires one inherited R5 switch."""

import argparse
import hashlib
import json
from pathlib import Path

import i07c_rd_sweep as sweep
from i07c_rd_drive6 import CORPORA, FLAGS


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("binary")
    parser.add_argument("piece", choices=("retract", "atomic0"))
    parser.add_argument("--limit", type=int, default=3000)
    args = parser.parse_args()
    binary_sha = hashlib.sha256(Path(args.binary).read_bytes()).hexdigest()
    if binary_sha != "82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb":
        raise RuntimeError(f"unexpected R5 binary {binary_sha}")
    complete_tag = "proto-" + binary_sha[:12] + "-r5owner"
    ablated_flags = ",".join(flag for flag in FLAGS.split(",") if flag != args.piece)
    for corpus in CORPORA:
        if corpus.startswith("b-projections") or corpus.startswith("owner-projections"):
            continue
        sources = sweep.data(corpus)
        try:
            complete = sweep.load(complete_tag, corpus)
        except FileNotFoundError:
            continue
        oracle = sweep.expected(corpus, sources)
        inspected = 0
        for index in sorted(complete):
            if inspected >= args.limit:
                break
            if oracle[index] is None:
                continue
            inspected += 1
            ablated = sweep.run_one((args.binary, ablated_flags,
                                     sweep.profile(corpus), sources[index]))
            if isinstance(ablated, dict):
                raise RuntimeError(f"INCONCLUSO {corpus} {index}: {ablated}")
            reference = complete[index]
            if ablated == reference:
                continue
            expected = oracle[index]
            if [r[:4] for r in reference] != [r[:4] for r in expected]:
                continue
            if [r[:4] for r in ablated] == [r[:4] for r in expected]:
                continue
            result = {"piece": args.piece, "corpus": corpus, "index": index,
                      "source": sweep.source(sources[index]), "oracle": expected,
                      "complete": reference, "ablated": ablated,
                      "complete_matches_core": [r[:4] for r in reference] == [r[:4] for r in expected],
                      "ablated_matches_core": [r[:4] for r in ablated] == [r[:4] for r in expected],
                      "binary_sha256": binary_sha, "flags": FLAGS}
            path = Path("docs/validation/i07c-rediseno") / f"r5-ablation-{args.piece}.json"
            path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print(json.dumps(result, ensure_ascii=False))
            return
        print(json.dumps({"piece": args.piece, "corpus": corpus, "inspected": inspected,
                          "difference": False}), flush=True)
    raise SystemExit("no discriminating source found within the bounded scan")


if __name__ == "__main__":
    main()
