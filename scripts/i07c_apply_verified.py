#!/usr/bin/env python3
"""Actualiza un testigo de mutante solo si la prueba exacta lo detectó."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("verification", type=Path)
    args = parser.parse_args()
    entries = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    results = DIRECTORY / "results.jsonl"
    rows = {row["id"]: row for row in map(json.loads, results.read_text(encoding="utf-8").splitlines())}
    assert len(rows) == len(entries)
    applied = []
    seen = set()
    for evidence in map(json.loads, args.verification.read_text(encoding="utf-8").splitlines()):
        if evidence["status"] not in {"detected", "detected_timeout"}:
            continue
        assert evidence["id"] not in seen, evidence["id"]
        seen.add(evidence["id"])
        assert evidence["test"] and evidence["test"] in evidence["witness"]
        assert evidence["baseline_seconds"] > 0
        if evidence["status"] == "detected_timeout":
            assert evidence["baseline_seconds"] < 10, evidence["id"]
        item = next(item for item in entries if item["id"] == evidence["id"])
        source_sha = hashlib.sha256((ROOT / item["file"]).read_bytes()).hexdigest()
        assert evidence["source_sha256"] == source_sha, evidence["id"]
        row = rows[evidence["id"]]
        assert row["result"]["release"] in {"superviviente", "detectada", "inconclusa"}
        row["result"]["release"] = "detectada"
        witness = evidence["witness"]
        if evidence["test"] not in witness:
            witness = (f"{evidence['test']}: no termina con el plazo medido"
                       if evidence["status"] == "detected_timeout" else evidence["test"])
        elif evidence["status"] == "detected_timeout" and "no termina" not in witness:
            witness = (f"{witness}: no termina en 30 s; "
                       f"base {evidence['baseline_seconds']:.3f} s")
        row.setdefault("witness", {})["release"] = witness
        rechecks = row.setdefault("rechecks", [])
        rechecks[:] = [check for check in rechecks
                       if (check.get("test"), check.get("profile")) != (evidence["test"], "release")]
        rechecks.append({
            "test": evidence["test"], "profile": "release", "result": "detectada",
            "witness": witness,
        })
        applied.append(evidence["id"])
    staging = results.with_suffix(".jsonl.tmp")
    staging.write_text("".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                               for item in entries), encoding="utf-8", newline="\n")
    staging.replace(results)
    print(f"Testigos exactos incorporados: {len(applied)} {','.join(applied)}")


if __name__ == "__main__":
    main()
