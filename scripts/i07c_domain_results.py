#!/usr/bin/env python3
"""Registra las campañas de dominios pendientes y sus gemelos concretos."""

import argparse
import hashlib
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/i07c-domain-results.json"
HEADER = re.compile(
    r"seed=(\d+) n=(\d+) depth=(\d+) acc_both=(\d+) rej_both=(\d+) "
    r"bugs=(\d+) dup=(\d+) syn=(\d+) skipped=(\d+)"
)
CLASS = re.compile(r"  (extra_in_deferred|missing_in_deferred):([ADV]): (\d+)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", nargs=4, type=Path, required=True)
    parser.add_argument("--debug", nargs=4, type=Path, required=True)
    args = parser.parse_args()
    campaigns = []
    for profile, paths in (("release", args.release), ("debug", args.debug)):
        for path in paths:
            raw = path.read_bytes()
            content = raw.decode("utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
            match = HEADER.search(content)
            assert match and content.count("seed=") == 1, path
            seed, programs, depth, accepted, rejected, bugs, duplicates, syntax, skipped = (
                int(value) for value in match.groups()
            )
            assert depth == 3 and programs == 100_000 and bugs == duplicates == syntax == 0
            assert accepted + rejected + skipped == programs
            classes = {f"{side}:{level}": int(count) for side, level, count in CLASS.findall(content)}
            assert len(classes) == len(CLASS.findall(content))
            assert not re.search(r"(?m)^(BUG|DUP|SIG .*\"U\")", content)
            campaigns.append({
                "profile": profile,
                "seed": seed,
                "programs": programs,
                "comparable": accepted + rejected,
                "accepted": accepted,
                "rejected": rejected,
                "skipped": skipped,
                "bugs": bugs,
                "duplicates": duplicates,
                "classes": classes,
                "log_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            })
    assert len({row["seed"] for row in campaigns}) == 8
    for profile in ("release", "debug"):
        rows = [row for row in campaigns if row["profile"] == profile]
        assert sum(row["comparable"] for row in rows) >= 300_000
    source = (ROOT / "src/check/types.rs").read_bytes().replace(b"\r\n", b"\n")
    result = {
        "source_sha256": hashlib.sha256(source).hexdigest(),
        "rules": {
            "A": "F0-0002 §4.1.2.7: the advanced initializer changes an incompatible accepted prefix",
            "D": "F0-0002 §4.1.2.4: additional leaf of the same inferred equality",
            "V": "F0-0002 §4.1.2.5: dependent result after a failed operand or guard",
        },
        "campaigns": campaigns,
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print("Campañas de dominio registradas:", len(campaigns))


if __name__ == "__main__":
    main()
