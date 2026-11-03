#!/usr/bin/env python3
"""Reclassify obsolete detected labels from current suite and differential evidence."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--differential", type=Path)
    parser.add_argument("--runtime-differential", type=Path)
    parser.add_argument("--exact-evidence", type=Path, action="append", default=[])
    parser.add_argument("--suite-isolation", type=Path)
    args = parser.parse_args()
    differential = ({row["id"]: row for row in map(json.loads,
        args.differential.read_text(encoding="utf-8").splitlines())}
        if args.differential else {})
    runtime = ({row["id"]: row for row in map(json.loads,
        args.runtime_differential.read_text(encoding="utf-8").splitlines())}
        if args.runtime_differential else {})
    exact = {}
    for path in args.exact_evidence:
        for row in map(json.loads, path.read_text(encoding="utf-8").splitlines()):
            exact.setdefault(row["id"], []).append(row)
    suite = (json.loads(args.suite_isolation.read_text(encoding="utf-8"))
             if args.suite_isolation else None)
    population = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    by_id = {item["id"]: item for item in population}
    path = DIRECTORY / "results.jsonl"
    rows = {row["id"]: row for row in map(json.loads, path.read_text(encoding="utf-8").splitlines())}
    assert len(rows) == len(population)
    changed = []
    for evidence in map(json.loads, args.evidence.read_text(encoding="utf-8").splitlines()):
        id = evidence["id"]
        assert id in by_id
        assert evidence["source_sha256"] == hashlib.sha256(
            (ROOT / by_id[id]["file"]).read_bytes()).hexdigest(), id
        status = evidence["status"]
        if status not in {"no_failure", "no_fast_failure", "bounded_equal", "full_suite_equal"}:
            continue
        if status in {"no_fast_failure", "bounded_equal", "full_suite_equal"}:
            assert all(measured[id]["status"] == "same" and measured[id]["differences"] == 0
                       for measured in (differential, runtime)), id
        if status == "bounded_equal":
            checks = exact.get(id, [])
            assert len({check["test"] for check in checks}) >= 2, id
            assert all(check["status"] == "not_killed"
                       and check["baseline_seconds"] < 10
                       and check["source_sha256"] == evidence["source_sha256"]
                       for check in checks), id
        if status == "full_suite_equal":
            assert suite is not None and suite["id"] == id
            assert suite["baseline"] == suite["mutated"] == 0
            assert suite["source_sha256"] == evidence["source_sha256"]
        row = rows[id]
        assert row["result"]["release"] in {"detectada", "inconclusa"}, id
        row["result"]["release"] = "superviviente"
        witness = ({
            "bounded_equal": "dos pruebas individuales sin diferencia; diferencial fijo same/0",
            "full_suite_equal": "suite completa release base/mutante sin fallo; diferencial fijo same/0",
            "no_fast_failure": "biblioteca e integraciones acotadas release sin fallo; diferencial fijo same/0",
            "no_failure": "biblioteca e integración release actuales sin fallo",
        }[status])
        row.setdefault("witness", {})["release"] = witness
        row.setdefault("rechecks", []).append({
            "test": "pruebas individuales y diferencial fijo" if status == "bounded_equal" else
                    "suite release completa y diferencial fijo" if status == "full_suite_equal" else
                    "batería acotada y diferencial fijo" if status == "no_fast_failure" else
                    "suite release completa",
            "profile": "release", "result": "superviviente", "witness": witness,
        })
        changed.append(id)
    path.write_text("".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                            for item in population), encoding="utf-8", newline="\n")
    print(f"Etiquetas corregidas con evidencia actual: {','.join(changed)}")


if __name__ == "__main__":
    main()
