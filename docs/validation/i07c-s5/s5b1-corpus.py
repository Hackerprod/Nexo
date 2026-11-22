"""Deterministic selector-family corpus for S5b.1; freeze before CLI use.

Usage: python -B docs/validation/i07c-s5/s5b1-corpus.py --freeze
       python -B docs/validation/i07c-s5/s5b1-corpus.py --verify
"""

import argparse
import hashlib
import json
from pathlib import Path
import random


HERE = Path(__file__).resolve().parent
SEED = 20261121
BASES = (
    "ar[true]", "(1+true)", "ar[true][0]", "ar[0]", "ar[0][0]",
    "ar[false]", "ar[true][2]", "ar[0][false]", "(ar[true])", "s[0]",
)
SELECTORS = (
    "[1,true]", "2+true", "0", "2", "[1,2]", "true", "(1+true)",
    "([1,true])", "[2,false]", "(2+2)",
)


def generated():
    cases = [
        f"fn Main(){{let ar=[1];var s=[];let n={n};let z={base}[{selector}];s=[[3]];}}"
        for n in range(30) for base in BASES for selector in SELECTORS
    ]
    random.Random(SEED).shuffle(cases)
    assert len(cases) == 3000 and len(set(cases)) == 3000
    return cases


def digest(cases):
    return hashlib.sha256(b"".join(
        len(source.encode("ascii")).to_bytes(4, "big") + source.encode("ascii")
        for source in cases
    )).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--freeze", action="store_true")
    mode.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    cases = generated()
    metadata = {"seed": SEED, "count": len(cases), "sha256": digest(cases),
                "encoding": "length-prefixed ASCII, four-byte big-endian length, in shuffled order"}
    manifest = HERE / "s5b1-corpus.json"
    if args.freeze:
        if manifest.exists():
            raise SystemExit("manifest already frozen; refusing to overwrite")
        manifest.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    else:
        assert json.loads(manifest.read_text(encoding="utf-8")) == metadata
    print(json.dumps(metadata, sort_keys=True))


if __name__ == "__main__":
    main()
