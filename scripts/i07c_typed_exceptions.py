#!/usr/bin/env python3
"""Check that every reduced typed/free twin difference has an ADR adjudication."""

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs/validation/i07c-typed-exceptions.json"
OUTPUT = ROOT / "docs/validation/i07c-typed-campaign-results.json"


def measured(path: Path) -> tuple[int, set[int]]:
    raw = path.read_bytes()
    text = raw.decode("utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8")
    header = re.search(r"programs=300000 seed=(\d+) depth=3 flagged=(\d+)", text)
    assert header, f"missing complete campaign header: {path}"
    seed, count = map(int, header.groups())
    assert "bugs=0" in text and "dups=0" in text, path
    assert not re.search(r"(?m)^(BUG|DUP) idx=", text), path
    ids = {int(value) for value in re.findall(r"(?m)^#\d+ idx=(\d+) ", text)}
    assert len(ids) == count, (path, count, len(ids))
    distinct = re.search(r"distinct reduced signatures=(\d+)", text)
    assert distinct and int(distinct.group(1)) == count, path
    return seed, ids


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", type=Path, nargs="+", help="complete typed/free campaign logs")
    args = parser.parse_args()
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    assert "§4.1.2.7" in data["rule"]
    expected: dict[int, set[int]] = {}
    for case in data["cases"]:
        assert case["prefix"].strip(), case
        seed, index = case["seed"], case["index"]
        bucket = expected.setdefault(seed, set())
        assert index not in bucket, case
        bucket.add(index)
    measured_by_seed = dict(measured(path) for path in args.logs)
    assert measured_by_seed == expected, {
        seed: {
            "missing": sorted(ids - measured_by_seed.get(seed, set())),
            "unclassified": sorted(measured_by_seed.get(seed, set()) - ids),
        }
        for seed, ids in expected.items()
    }
    profiles = data["profiles"]
    assert set(profiles) == {str(seed) for seed in expected}
    source = (ROOT / "src/check/types.rs").read_bytes().replace(b"\r\n", b"\n")
    output = {
        "source_sha256": hashlib.sha256(source).hexdigest(),
        "adjudication_sha256": hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
        "campaigns": [
            {
                "profile": profiles[str(seed)],
                "seed": seed,
                "programs": 300_000,
                "reduced_signatures": len(ids),
                "adjudicated": len(ids),
                "bugs": 0,
                "duplicates": 0,
            }
            for seed, ids in sorted(measured_by_seed.items())
        ],
    }
    OUTPUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print(f"Adjudicated typed/free signatures: {sum(map(len, expected.values()))}")


if __name__ == "__main__":
    main()
