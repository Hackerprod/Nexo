#!/usr/bin/env python3
"""Keep one exact and current witness for every detected production mutant."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    population = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    results = {row["id"]: row for row in map(json.loads,
        (DIRECTORY / "results.jsonl").read_text(encoding="utf-8").splitlines())}
    detected = {item["id"] for item in population
                if results[item["id"]]["result"]["release"] == "detectada"}
    evidence = {}
    for path in args.inputs:
        for row in map(json.loads, path.read_text(encoding="utf-8").splitlines()):
            if row["status"] in {"detected", "detected_timeout"}:
                evidence[row["id"]] = row
    assert set(evidence) == detected, sorted(detected - set(evidence))
    for item in population:
        id = item["id"]
        if id not in detected:
            continue
        row = evidence[id]
        assert row["test"] in results[id]["witness"]["release"], id
        assert row["source_sha256"] == hashlib.sha256((ROOT / item["file"]).read_bytes()).hexdigest(), id
        if row["status"] == "detected_timeout":
            assert row["baseline_seconds"] < 10, id
    ordered = [evidence[item["id"]] for item in population if item["id"] in detected]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, ensure_ascii=False) + "\n" for row in ordered),
                           encoding="utf-8", newline="\n")
    print(f"Citas exactas revalidadas: {len(ordered)}")


if __name__ == "__main__":
    main()
