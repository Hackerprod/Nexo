#!/usr/bin/env python3
"""Merge disposable projection shards in population order after adjudication."""

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("projection", "projection_full", "projection_runtime"), required=True)
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    population = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    results = {row["id"]: row for row in map(json.loads,
        (DIRECTORY / "results.jsonl").read_text(encoding="utf-8").splitlines())}
    survivors = {item["id"] for item in population
                 if results[item["id"]]["result"]["release"] == "superviviente"}
    rows = {}
    for path in args.inputs:
        for row in map(json.loads, path.read_text(encoding="utf-8").splitlines()):
            assert row["mode"] == args.mode, (path, row["id"])
            if row["status"] == "normative_difference":
                row["status"] = "raw_difference"
            if row["status"] == "same":
                assert row["differences"] == 0
                row.setdefault("changed_families", {})
            if row["status"] == "raw_difference":
                assert row["differences"] > 0
            rows[row["id"]] = row
    assert survivors <= rows.keys(), sorted(survivors - rows.keys())
    assert all(rows[id]["status"] in {"same", "raw_difference"} for id in survivors)
    ordered = [rows[item["id"]] for item in population if item["id"] in survivors]
    assert all(sum(row.get("changed_families", {}).values()) == row["differences"]
               for row in ordered if row["status"] == "raw_difference")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, ensure_ascii=False) + "\n" for row in ordered),
                           encoding="utf-8", newline="\n")
    print(f"{args.mode}: {len(ordered)} supervivientes, "
          f"{sum(row['status'] == 'raw_difference' for row in ordered)} diferencias crudas")


if __name__ == "__main__":
    main()
