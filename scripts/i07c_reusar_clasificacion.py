#!/usr/bin/env python3
"""Conserva argumentos solo para sustituciones con contexto idéntico y supervivientes."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


FIELDS = ("file", "operator", "literal", "replacement")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("old_population", type=Path)
    parser.add_argument("old_arguments", type=Path)
    parser.add_argument("new_population", type=Path)
    parser.add_argument("new_results", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    old = {item["id"]: item for item in
           json.loads(args.old_population.read_text(encoding="utf-8"))["mutations"]}
    current = {tuple(item[field] for field in FIELDS): item for item in
               json.loads(args.new_population.read_text(encoding="utf-8"))["mutations"]}
    results = {row["id"]: row for row in
               map(json.loads, args.new_results.read_text(encoding="utf-8").splitlines())}
    arguments = json.loads(args.old_arguments.read_text(encoding="utf-8"))
    kept = {}
    for old_id, argument in arguments.items():
        source = old[old_id]
        candidate = current.get(tuple(source[field] for field in FIELDS))
        if candidate is None:
            continue
        new_id = candidate["id"]
        if results[new_id]["result"].get("release") != "superviviente":
            continue
        text = Path(candidate["file"]).read_text(encoding="utf-8")
        assert text.count(candidate["literal"]) == 1, new_id
        assert hashlib.sha256(candidate["literal"].encode()).hexdigest() == hashlib.sha256(source["literal"].encode()).hexdigest()
        assert new_id not in kept
        kept[new_id] = argument
    ordered = dict(sorted(kept.items()))
    args.output.write_text(json.dumps(ordered, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8", newline="\n")
    print(f"Argumentos conservados: {len(ordered)}/{len(arguments)}")


if __name__ == "__main__":
    main()
