#!/usr/bin/env python3
"""Regenera la población I10b con codificación y orden canónicos."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
POPULATION = ROOT / "docs/validation/F0-0002-I10b-mutation-population.json"


def canonical_bytes() -> bytes:
    document = json.loads(POPULATION.read_text(encoding="utf-8"))
    if document.get("schema") != "i10b-mutation-population-v1":
        raise ValueError("unexpected mutation population schema")
    mutants = document.get("mutants")
    if not isinstance(mutants, list) or len(mutants) < 90:
        raise ValueError("mutation population must contain at least 90 rows")
    identifiers = [row.get("id") for row in mutants]
    if len(set(identifiers)) != len(identifiers) or identifiers != sorted(identifiers):
        raise ValueError("mutation identifiers must be unique and sorted")
    document["mutants"] = sorted(mutants, key=lambda row: row["id"])
    return (json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def main() -> None:
    POPULATION.write_bytes(canonical_bytes())


if __name__ == "__main__":
    main()
