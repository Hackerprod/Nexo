#!/usr/bin/env python3
"""Traslada resultados solo cuando el contexto literal de la mutación es idéntico."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def key(item: dict) -> tuple[str, str, str, str]:
    return item["file"], item["operator"], item["literal"], item["replacement"]


def read_prior(population: Path, results: Path) -> dict[tuple[str, str, str, str], dict]:
    entries = {item["id"]: item for item in
               json.loads(population.read_text(encoding="utf-8"))["mutations"]}
    accepted: dict[tuple[str, str, str, str], dict] = {}
    for line in results.read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        item = entries[row["id"]]
        if row["mutation_sha256"] != digest(item):
            continue
        context = key(item)
        assert context not in accepted, context
        accepted[context] = row
    return accepted


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("population", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("prior", type=Path, nargs="+", help="pares población/resultados")
    args = parser.parse_args()
    assert len(args.prior) % 2 == 0
    current = json.loads(args.population.read_text(encoding="utf-8"))["mutations"]
    sources = [read_prior(pop, results) for pop, results in
               zip(args.prior[::2], args.prior[1::2])]
    kept = []
    counts = [0] * len(sources)
    for item in current:
        context = key(item)
        source = next(((index, rows[context]) for index, rows in enumerate(sources)
                       if context in rows), None)
        if source is None:
            continue
        index, prior = source
        # The literal is the unique enclosing source context, including every
        # token needed to apply this substitution at exactly one location.
        text = Path(item["file"]).read_text(encoding="utf-8")
        assert text.count(item["literal"]) == 1, item["id"]
        row = dict(prior)
        row["id"] = item["id"]
        row["mutation_sha256"] = digest(item)
        row["context_sha256"] = hashlib.sha256(item["literal"].encode()).hexdigest()
        kept.append(row)
        counts[index] += 1
    args.output.write_text("".join(json.dumps(row, ensure_ascii=False) + "\n"
                                   for row in kept), encoding="utf-8", newline="\n")
    print(f"Contextos idénticos: {len(kept)}/{len(current)}; por origen: {counts}")


if __name__ == "__main__":
    main()
