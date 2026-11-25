#!/usr/bin/env python3
"""Run one frozen corpus across the transaction probes under one heavy lock."""
import argparse
import contextlib
import hashlib
import io
import json
from pathlib import Path

import i07c_rd_sweep as sweep

BASE_FLAGS = "pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr,skipv3"
VARIANTS = {
    "right": "txmarks,txmarksright",
    "concrete": "txmarks,txmarksconcrete",
    "statement": "txmarks,txmarksstatement",
    "occurrence": "txmarks,txmarksoccurrence",
    "statement_right": "txmarks,txmarksstatement,txmarksright",
    "occurrence_right": "txmarks,txmarksoccurrence,txmarksright",
    "occurrence_unify": "txmarks,txmarksoccurrence,txunify",
    "projection": "txmarks,txmarksproj",
    "r5": "r5prefix,r1t5,r5t5",
}
QUALIFIED = tuple(VARIANTS)


def run(tag, corpus, exe, flags, start, end):
    dest = sweep.OUT / tag / f"{corpus}-{start:05d}-{end:05d}.json"
    if dest.exists():
        old = json.loads(dest.read_text(encoding="utf-8"))
        results = old.get("results", [])
        if (old.get("start") == start and old.get("end") == end and old.get("flags") == flags
                and len(results) == end - start and all(not isinstance(value, dict) for value in results)):
            print(f"cached={dest}", flush=True)
            return
    args = argparse.Namespace(tag=tag, corpus=corpus, exe=exe, flags=flags, start=start, end=end)
    sweep.run(args)


def table(base, candidate, corpus):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        sweep.table(argparse.Namespace(base=base, candidate=candidate, corpus=corpus))
    return json.loads(out.getvalue())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus")
    parser.add_argument("base_exe")
    parser.add_argument("proto_exe")
    parser.add_argument("--variants", default=",".join(QUALIFIED))
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--end", type=int)
    parser.add_argument("--series", default="")
    args = parser.parse_args()
    args.base_exe = str(__import__("pathlib").Path(args.base_exe).resolve())
    args.proto_exe = str(__import__("pathlib").Path(args.proto_exe).resolve())
    selected = args.variants.split(",")
    if any(name not in VARIANTS for name in selected):
        parser.error("unknown variant")
    count = len(sweep.data(args.corpus))
    end = min(args.end if args.end is not None else count, count)
    if args.start < 0 or args.start >= end:
        parser.error("invalid lot range")
    print(f"corpus={args.corpus} rows={count} lot={args.start}:{end} variants={selected}", flush=True)
    hashes = {}
    for role, exe in (("base", args.base_exe), ("prototype", args.proto_exe)):
        with open(exe, "rb") as stream:
            hashes[role] = hashlib.file_digest(stream, "sha256").hexdigest()
    label = (args.series + "-" if args.series else "") + args.corpus
    published = Path("docs/validation/i07c-rediseno/barrido") / ("summary-" + label + ".json")
    published.parent.mkdir(parents=True, exist_ok=True)
    if published.exists():
        summary = json.loads(published.read_text(encoding="utf-8"))
        if summary.get("corpus") != args.corpus or summary.get("rows") != count or summary.get("binary_sha256") != hashes:
            raise RuntimeError("published summary belongs to different corpus or binaries")
    else:
        summary = {"corpus": args.corpus, "rows": count, "binary_sha256": hashes, "variants": {}}
    base_tag = "main-" + hashes["base"][:12]
    proto_tag = "proto-" + hashes["prototype"][:12] + "-"
    run(base_tag, args.corpus, args.base_exe, None, args.start, end)
    for name in selected:
        run(proto_tag + name, args.corpus, args.proto_exe,
            BASE_FLAGS + "," + VARIANTS[name], args.start, end)
        result = table(base_tag, proto_tag + name, args.corpus)
        if result["n"] < end - args.start:
            raise RuntimeError(f"incomplete lot {args.corpus}: {result['n']}/{end - args.start}")
        summary["variants"][name] = result
        path = sweep.OUT / ("summary-" + label + ".json")
        serialized = json.dumps(summary, ensure_ascii=False, indent=2)
        path.write_text(serialized, encoding="utf-8")
        published.write_text(serialized, encoding="utf-8")
        compact = {k: v for k, v in result.items() if k != "regressions"}
        compact["regressions_sample"] = result["regressions"][:10]
        print(json.dumps({"variant": name, **compact}, ensure_ascii=False), flush=True)
    print(f"completed={args.corpus}", flush=True)


if __name__ == "__main__":
    main()
