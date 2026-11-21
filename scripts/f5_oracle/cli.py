"""Command line entry for the independent F5 oracle."""

from __future__ import annotations

import argparse
import json

from . import selftest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("selftest", help="check normative fixtures and reference rules")
    args = parser.parse_args()
    if args.command == "selftest":
        print(json.dumps(selftest.run(), sort_keys=True))
        return 0
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
