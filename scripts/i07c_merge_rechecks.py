#!/usr/bin/env python3
"""Integra las reejecuciones dirigidas sobre la población cerrada de I07c."""

import argparse
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from i07c_mutaciones import OUTPUT, candidates, checked_results, shard_path


def write_rows(path: Path, entries: list[dict], rows: dict[str, dict]) -> None:
    staging = path.with_suffix(path.suffix + ".tmp")
    staging.write_text(
        "".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n" for item in entries),
        encoding="utf-8",
        newline="\n",
    )
    staging.replace(path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="b506d7d")
    parser.add_argument("--recheck", type=Path, action="append", required=True,
                        help="resultado aislado con testigos dirigidos; se puede repetir")
    args = parser.parse_args()
    directory = OUTPUT.parent
    population = json.loads((directory / "population.json").read_text(encoding="utf-8"))
    assert population["base"] == args.base
    entries = population["mutations"]
    assert entries == candidates(args.base)
    rows = checked_results(directory / "results.jsonl", entries)
    assert len(rows) == len(entries), "el barrido completo debe terminar antes de integrar"
    selected = {}
    for path in args.recheck:
        for id, candidate in checked_results(path, entries).items():
            rechecks = candidate.get("rechecks", [])
            if not rechecks:
                continue
            assert id in rows
            assert rows[id]["mutation_sha256"] == candidate["mutation_sha256"]
            assert rows[id]["result"]["release"] in ("superviviente", "inconclusa", "detectada")
            assert all(check["profile"] == "release" and check["test"] and check["witness"]
                       for check in rechecks)
            assert candidate["result"]["release"] == rechecks[-1]["result"]
            detected = next((check for check in rechecks if check["result"] == "detectada"), None)
            if detected is not None:
                candidate["result"]["release"] = "detectada"
                candidate["witness"]["release"] = detected["witness"]
            elif rows[id]["result"]["release"] == "detectada":
                candidate["result"]["release"] = "detectada"
                candidate["witness"]["release"] = rows[id]["witness"]["release"]
            if id in selected:
                prior = selected[id]
                assert prior["mutation_sha256"] == candidate["mutation_sha256"]
                merged = {(check["test"], check["profile"]): check
                          for check in prior["rechecks"] + rechecks}
                candidate["rechecks"] = list(merged.values())
                if prior["result"]["release"] == "detectada":
                    candidate["result"]["release"] = "detectada"
                    candidate["witness"]["release"] = prior["witness"]["release"]
            selected[id] = candidate
    rows.update(selected)
    assert len(rows) == len(entries)
    write_rows(directory / "results.jsonl", entries, rows)
    for shard in range(2):
        write_rows(shard_path(shard, 2), entries[shard::2], rows)
    print(f"Reejecuciones integradas: {len(selected)}; población: {len(entries)}")


if __name__ == "__main__":
    main()
