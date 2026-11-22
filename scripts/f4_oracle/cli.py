"""Command line of the F4 oracle (class ORQUESTA).

``python -B -m scripts.f4_oracle.cli selftest [--fast] [--groups G0,G1]`` and ``audit``.
The comparison commands (generate, fuzz-corpus, compare, fuzz, minimize, gate, batch)
arrive with the later commits of the plan.
"""

from __future__ import annotations

import argparse
import sys

import platform

from . import audit, fixtures_io, generate, selftest


def _corpus_sha(args) -> int:
    """Print or pin the SHA-256 of the generated corpus per label, seed and class."""
    if args.write:
        table = selftest.corpus_table()
        doc = {"version": 1, "python": platform.python_version(),
               "scale": {k: list(v) for k, v in selftest.SCALE.items()}, "g10": [1000, 1000, 50],
               "note": "SHA-256 of the canonical JSON records per label|seed|class (V, N, L, B); "
                       "regenerate only with `corpus-sha --write` after reviewing the generator change.",
               "entries": table}
        path = fixtures_io.fixture_path("f4_oracle_corpus_sha.json")
        with open(path, "wb") as handle:
            handle.write((fixtures_io.dumps(doc) + chr(10)).encode("utf-8"))
        print(f"corpus-sha: wrote {len(table)} entries")
        return 0
    for label, phase, complete, nv, ni, seeds, _ in selftest.gen_labels():
        if args.label and label != args.label:
            continue
        for seed in seeds:
            if args.seeds:
                lo, _, hi = args.seeds.partition(":")
                if not (int(lo) <= seed <= int(hi or lo)):
                    continue
            recs, _, _ = generate.generate_seed(seed, phase, complete, nv, ni)
            for c in "VNLB":
                print(f"{label}|{seed}|{c}={generate.corpus_sha(recs, c)}")
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="f4_oracle")
    sub = parser.add_subparsers(dest="command", required=True)
    st = sub.add_parser("selftest")
    st.add_argument("--fast", action="store_true")
    st.add_argument("--groups", default="")
    sub.add_parser("audit")
    cs = sub.add_parser("corpus-sha")
    cs.add_argument("--label", default="")
    cs.add_argument("--seeds", default="")
    cs.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    if args.command == "corpus-sha":
        return _corpus_sha(args)
    if args.command == "audit":
        findings = audit.run()
        for item in findings:
            print(f"AUDIT {item}")
        print(f"audit: {len(findings)} findings")
        return 1 if findings else 0
    if args.command == "selftest":
        groups = [g for g in args.groups.split(",") if g] or None
        return selftest.run(fast=args.fast, groups=groups)
    return 2


if __name__ == "__main__":
    sys.exit(main())
