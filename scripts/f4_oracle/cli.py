"""Command line of the F4 oracle (class ORQUESTA).

``python -B -m scripts.f4_oracle.cli selftest [--fast] [--groups G0,G1]`` and ``audit``.
The comparison commands (generate, fuzz-corpus, compare, fuzz, minimize, gate, batch)
arrive with the later commits of the plan.
"""

from __future__ import annotations

import argparse
import sys

from . import audit, selftest


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="f4_oracle")
    sub = parser.add_subparsers(dest="command", required=True)
    st = sub.add_parser("selftest")
    st.add_argument("--fast", action="store_true")
    st.add_argument("--groups", default="")
    sub.add_parser("audit")
    args = parser.parse_args(argv)
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
