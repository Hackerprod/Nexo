#!/usr/bin/env python3
"""Deriva la clase de cada superviviente de un diferencial normativo completo."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--differential", type=Path,
                        default=DIRECTORY / "normative-differential.jsonl")
    parser.add_argument("--runtime-differential", type=Path,
                        default=DIRECTORY / "normative-runtime-differential.jsonl")
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--base-projection", type=Path, required=True)
    parser.add_argument("--accepted-corpus", type=Path, required=True)
    parser.add_argument("--runtime-baseline", type=Path, required=True)
    args = parser.parse_args()
    population = read_json(DIRECTORY / "population.json")["mutations"]
    by_id = {item["id"]: item for item in population}
    assert len(by_id) == len(population)
    results = [json.loads(line) for line in
               (DIRECTORY / "results.jsonl").read_text(encoding="utf-8").splitlines()]
    assert [row["id"] for row in results] == [item["id"] for item in population]
    survivors = {row["id"] for row in results
                 if row["result"]["release"] == "superviviente"}
    equivalent = set(read_json(DIRECTORY / "equivalence.json"))
    i10a = set(read_json(DIRECTORY / "i10a.json"))
    assert not equivalent & i10a
    assert equivalent | i10a <= survivors
    informative = survivors - equivalent - i10a
    assert all(item in by_id for item in survivors)

    corpus = args.corpus.read_bytes()
    base = args.base_projection.read_bytes()
    lines = corpus.splitlines()
    unique_programs = len(set(lines))
    assert len(lines) >= 400_000 and unique_programs >= 400_000
    assert len(base.splitlines()) == len(lines)
    assert not any(line.startswith(b"BUG:") for line in base.splitlines())
    accepted = args.accepted_corpus.read_bytes()
    accepted_lines = accepted.splitlines()
    assert accepted_lines == [source for source, result in zip(lines, base.splitlines(), strict=True)
                              if result == b"Ok"]
    runtime_base = args.runtime_baseline.read_bytes()
    assert len(runtime_base.splitlines()) == len(accepted_lines)
    assert all(line == b"A" for line in runtime_base.splitlines())

    differential = [json.loads(line) for line in args.differential.read_text(encoding="utf-8").splitlines()]
    measured = {row["id"]: row for row in differential}
    assert len(measured) == len(differential)
    assert survivors <= measured.keys(), sorted(survivors - measured.keys())
    assert all(measured[item]["mode"] == "projection_full" for item in survivors)
    assert all(measured[item]["status"] in {"same", "raw_difference"} for item in survivors)
    changed = {item for item in survivors if measured[item]["status"] == "raw_difference"}
    adjudicated = read_json(DIRECTORY / "informative-differences.json")
    assert set(adjudicated) == changed, sorted(changed - set(adjudicated))
    assert changed <= informative
    for item in changed:
        family_rules = adjudicated[item]
        assert set(family_rules) == set(measured[item]["changed_families"]), item
        assert sum(measured[item]["changed_families"].values()) == measured[item]["differences"]
        for family, rule in family_rules.items():
            assert rule["level"] in {"D", "V"}, (item, family)
            assert rule["adr"] and rule["reason"], (item, family)
    assert all(measured[item]["differences"] == 0 for item in survivors - changed)
    runtime = [json.loads(line) for line in args.runtime_differential.read_text(encoding="utf-8").splitlines()]
    runtime_measured = {row["id"]: row for row in runtime}
    assert len(runtime_measured) == len(runtime)
    assert survivors <= runtime_measured.keys(), sorted(survivors - runtime_measured.keys())
    assert all(runtime_measured[item]["mode"] == "projection_runtime" for item in survivors)
    runtime_unproved = [item for item in sorted(survivors)
                        if runtime_measured[item]["status"] != "same"
                        or runtime_measured[item]["differences"] != 0]
    assert not runtime_unproved, f"supervivientes con fallo de compilación/ejecución: {runtime_unproved}"
    evidence = {
        "base": "b506d7d",
        "programs": len(lines),
        "unique_programs": unique_programs,
        "corpus_sha256": hashlib.sha256(corpus).hexdigest(),
        "projection_mode": "projection_full",
        "base_projection_sha256": hashlib.sha256(base).hexdigest(),
        "accepted_programs": len(accepted_lines),
        "accepted_corpus_sha256": hashlib.sha256(accepted).hexdigest(),
        "runtime_baseline_sha256": hashlib.sha256(runtime_base).hexdigest(),
        "survivors_measured": len(survivors),
        "informative": len(informative),
        "equivalent": len(equivalent),
        "i10a": len(i10a),
        "survivor_normative_differences": 0,
        "survivors_with_informative_differences": len(changed),
        "informative_changed_programs": sum(measured[item]["differences"] for item in changed),
        "runtime_failures": 0,
    }
    (DIRECTORY / "informative.json").write_text(
        json.dumps(sorted(informative), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8", newline="\n")
    (DIRECTORY / "normative-classification.json").write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Clasificación derivada: {len(survivors)} supervivientes, {len(lines)} fuentes, 0 diferencias")


if __name__ == "__main__":
    main()
