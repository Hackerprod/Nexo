#!/usr/bin/env python3
"""Run one frozen corpus across the transaction probes under one heavy lock."""
import argparse
import contextlib
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
}


def run(tag, corpus, exe, flags, count):
    args = argparse.Namespace(tag=tag, corpus=corpus, exe=exe, flags=flags, start=0, end=count)
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
    parser.add_argument("--variants", default=",".join(VARIANTS))
    args = parser.parse_args()
    selected = args.variants.split(",")
    if any(name not in VARIANTS for name in selected):
        parser.error("unknown variant")
    count = len(sweep.data(args.corpus))
    print(f"corpus={args.corpus} rows={count} variants={selected}", flush=True)
    run("main-6e22181", args.corpus, args.base_exe, None, count)
    for name in selected:
        run("proto-" + name, args.corpus, args.proto_exe,
            BASE_FLAGS + "," + VARIANTS[name], count)
        result = table("main-6e22181", "proto-" + name, args.corpus)
        print(json.dumps({"variant": name, **result}, ensure_ascii=False), flush=True)
    print(f"completed={args.corpus}", flush=True)


if __name__ == "__main__":
    main()
