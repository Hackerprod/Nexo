#!/usr/bin/env python3
"""Render durable I07c corpus summaries without silently skipping incomplete sets."""
import argparse
import json

import i07c_rd_sweep as sweep

CORPORA = (
    "c1", "c2", "c2@f0-0003-b-v1", "b", "perm", "t",
    "s7021993", "s4417061", "s9908255", "dirigido5",
    "c1-nueva", "c2-nueva",
)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", help="comma-separated completed corpora")
    args = parser.parse_args()
    names = args.only.split(",") if args.only else CORPORA
    print("| Corpus | Variante | n | sin oráculo | nivel | OK | A | B | C | D |")
    print("|---|---|---:|---:|---|---:|---:|---:|---:|---:|")
    for name in names:
        path = sweep.OUT / ("summary-" + name + ".json")
        if not path.exists():
            print(f"| {name} | NO EVALUADO | {len(sweep.data(name))} | — | — | — | — | — | — | — |")
            continue
        doc = json.loads(path.read_text(encoding="utf-8"))
        expected = len(sweep.data(name))
        if doc["rows"] != expected:
            raise ValueError(f"{name}: summary says {doc['rows']}, frozen corpus has {expected}")
        for variant, result in doc["variants"].items():
            if result["n"] != expected:
                raise ValueError(f"{name}/{variant}: incomplete {result['n']}/{expected}")
            for level in ("core", "full"):
                values = result[level]
                print(f"| {name} | {variant} | {expected} | {result['unmodeled']} | {level} | "
                      f"{result['ok'][level]} | {values['A']} | {values['B']} | {values['C']} | {values['D']} |")


if __name__ == "__main__":
    main()
