"""Command line entry for the independent F5 oracle."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from . import selftest
from .runner import run_compare, run_fuzz


def _arguments(command) -> None:
    command.add_argument("--binary-debug", required=True, type=Path)
    command.add_argument("--binary-release", required=True, type=Path)
    command.add_argument("--profile", required=True)
    command.add_argument("--phase", required=True)
    command.add_argument("--complete", required=True)
    command.add_argument("--seeds", required=True)
    command.add_argument("--output", required=True, type=Path)
    command.add_argument("--timeout", type=float, default=10.0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("selftest", help="check normative fixtures and reference rules")
    compare = commands.add_parser("compare", help="compare generated sources with two binaries")
    _arguments(compare)
    compare.add_argument("--valid-per-seed", type=int, default=1000)
    compare.add_argument("--invalid-per-seed", type=int, default=1000)
    compare.add_argument("--no-literals", action="store_true")
    gate = commands.add_parser("gate", help="run selftest before a differential")
    _arguments(gate)
    gate.add_argument("--valid-per-seed", type=int, default=1000)
    gate.add_argument("--invalid-per-seed", type=int, default=1000)
    gate.add_argument("--no-literals", action="store_true")
    fuzz = commands.add_parser("fuzz", help="probe malformed bytes and prefixes")
    _arguments(fuzz)
    fuzz.add_argument("--cases-per-seed", type=int, default=100)
    args = parser.parse_args()
    if args.command == "selftest":
        print(json.dumps(selftest.run(), sort_keys=True))
        return 0
    if args.command == "gate":
        checks = selftest.run()
        files = ("model.py", "diagnostics.py", "charges.py", "generate.py",
                 "selftest.py", "runner.py", "cli.py")
        digest = hashlib.sha256()
        for name in files:
            digest.update((Path(__file__).parent / name).read_bytes())
        print(json.dumps({"selftest": checks, "oracle_sha256": digest.hexdigest()},
                         sort_keys=True), flush=True)
    if args.command in ("compare", "gate"):
        result = run_compare(binary_debug=args.binary_debug,
                             binary_release=args.binary_release,
                             profile=args.profile, phase=args.phase,
                             complete=args.complete, seeds=args.seeds,
                             valid_per_seed=args.valid_per_seed,
                             invalid_per_seed=args.invalid_per_seed,
                             output=args.output, timeout=args.timeout,
                             include_literals=not args.no_literals)
        print(json.dumps(result, sort_keys=True))
        return 1 if result["differences"] else 0
    if args.command == "fuzz":
        result = run_fuzz(binary_debug=args.binary_debug,
                          binary_release=args.binary_release,
                          profile=args.profile, phase=args.phase,
                          complete=args.complete, seeds=args.seeds,
                          cases_per_seed=args.cases_per_seed,
                          output=args.output, timeout=args.timeout)
        print(json.dumps(result, sort_keys=True))
        return 1 if result["failures"] else 0
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
