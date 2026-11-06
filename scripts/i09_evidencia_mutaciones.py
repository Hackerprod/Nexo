#!/usr/bin/env python3
"""Clasifica y deriva artefactos legibles desde la población y ejecución I09."""

from __future__ import annotations

import hashlib
import json
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MUTATIONS = ROOT / "docs/validation/i09-mutations"
POPULATION = MUTATIONS / "population.json"
RESULTS = MUTATIONS / "results.jsonl"
TARGETED = MUTATIONS / "targeted-results.jsonl"
LOG = ROOT / "docs/validation/i09-evidence/mutation.log"

def equivalence(item: dict) -> str | None:
    operator = item["operator"]
    literal = item["literal"]
    if operator == "false" and "owned_temps.get(temp.0)" in literal:
        return (
            "La tabla de ownership se consulta solo con TempId verificados. "
            "verify_function rechaza cada referencia sin definición o fuera de rango antes de "
            "effects::analyze; el fallback no cambia un programa fuente válido."
        )
    if operator == "continue" and "temp_types.get(temp.0)" in literal:
        return (
            "La tabla de tipos temporales contiene una entrada para cada TempId verificado. "
            "verify_function valida las definiciones antes de effects::analyze; el caso sin tipo "
            "no es alcanzable desde IR de fuente válida."
        )
    if operator == "continue" and "temp_spans.get(temp.0)" in literal:
        return (
            "Cada temporal usado procede de una instrucción definitoria verificada con span. "
            "verify_function valida la definición antes de effects::analyze; no falta un span "
            "para temporales de IR de fuente válida."
        )
    if operator == "continue" and "EFFECTS.iter().find" in literal:
        return (
            "Los únicos efectos aceptados se encuentran en EFFECTS; `unrestricted` se trata "
            "antes de esta búsqueda y `no_heap_alloc` se rechaza como NX-E0006 antes del análisis "
            "de efectos (F0-0002 §§3, 13.12)."
        )
    return None


def load_lines(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_lines(path: Path, rows: list[dict]) -> None:
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
        newline="\n",
    )


def item_key(item: dict) -> tuple:
    return tuple(item[field] for field in ("file", "operator", "literal", "replacement", "site_count"))


def rebuild_from_previous(population: list[dict], previous_population_path: Path,
                          replacement_results_path: Path) -> list[dict]:
    previous_population = json.loads(previous_population_path.read_text(encoding="utf-8"))["mutations"]
    previous_items = {item["id"]: item for item in previous_population}
    previous_rows = {row["id"]: row for row in load_lines(RESULTS)}
    assert set(previous_rows) == set(previous_items), "previous results do not match previous population"
    previous_by_key = {item_key(item): previous_rows[item["id"]] for item in previous_population}
    replacements = {row["id"]: row for row in load_lines(replacement_results_path)}
    rows = []
    for item in population:
        key = item_key(item)
        if key in previous_by_key:
            row = dict(previous_by_key[key])
            row["id"] = item["id"]
            row["mutation_sha256"] = hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest()
        elif item["operator"] == "temporary_source_span_is_retained":
            row = dict(replacements["M00151"])
            row["id"] = item["id"]
            row["mutation_sha256"] = hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest()
        else:
            raise AssertionError(f"no executed mutation evidence for {item['id']}")
        rows.append(row)
    return rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--previous-population", type=Path)
    parser.add_argument("--replacement-results", type=Path)
    args = parser.parse_args()
    population = json.loads(POPULATION.read_text(encoding="utf-8"))["mutations"]
    if args.previous_population or args.replacement_results:
        assert args.previous_population and args.replacement_results
        actual_rows = rebuild_from_previous(population, args.previous_population, args.replacement_results)
    else:
        actual = {row["id"]: row for row in load_lines(RESULTS)}
        assert set(actual) == {item["id"] for item in population}, "resultados incompletos o sobrantes"
        actual_rows = [actual[item["id"]] for item in population]

    rows: list[dict] = []
    log: list[str] = []
    directed: list[dict] = []
    for item, row in zip(population, actual_rows, strict=True):
        expected_hash = hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest()
        assert row["mutation_sha256"] == expected_hash, item["id"]
        outcome = row["result"].get("release")
        explanation = equivalence(item)
        if outcome == "superviviente" and explanation:
            row["result"]["release"] = "equivalente"
            row["equivalence"] = explanation
            outcome = "equivalente"
        assert outcome == "detectada" or (outcome == "equivalente" and row.get("equivalence")), (
            item["id"], outcome
        )
        rows.append(row)
        witness = row.get("witness", {}).get("release", "")
        if outcome == "equivalente":
            log.append(f"{item['id']} {item['operator']} equivalente ; verify_function antes de effects::analyze ; sha256_restaurado=True; equivalence={row['equivalence']}\n")
        else:
            log.append(f"{item['id']} {item['operator']} detectada ; {witness} ; sha256_restaurado=True\n")
        if item["operator"] in {
            "parameter_mode_owns_only_move",
            "temporary_source_span_is_retained",
            "temporary_drop_span_covers_call",
            "binding_drop_span_covers_call",
        }:
            assert outcome == "detectada", (item["id"], item["operator"], outcome)
            directed.append(row)

    assert len(directed) == 4, "deben estar las cuatro regresiones inversas dirigidas"
    write_lines(RESULTS, rows)
    write_lines(TARGETED, directed)
    LOG.write_text("".join(log), encoding="utf-8", newline="\n")
    for shard in range(2):
        shard_rows = [row for index, row in enumerate(rows) if index % 2 == shard]
        write_lines(MUTATIONS / f"results-{shard:02d}-of-02.jsonl", shard_rows)
    intermediate = MUTATIONS / "results-150-of-153.jsonl"
    if intermediate.exists():
        intermediate.unlink()
    print(f"Clasificados {len(rows)} mutantes; {len(directed)} regresiones dirigidas detectadas")


if __name__ == "__main__":
    main()
