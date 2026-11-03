#!/usr/bin/env python3
"""Regenera las atribuciones de pruebas desde la población medida."""

import json
import sys
from collections import defaultdict
from pathlib import Path

from i07c_mutaciones import is_test_only

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
MUTATIONS = ROOT / "docs/validation/i07c-mutations"
NOTES = ROOT / "docs/validation/F0-0002-I07c-implementation-notes.md"
START = "<!-- i07c-attributions:start -->"
END = "<!-- i07c-attributions:end -->"
TESTS = (
    "divergent_binding_read_preserves_unresolved_inference",
    "invalid_first_call_argument_preserves_independent_second_argument",
    "selector_equality_error_survives_non_array_base",
    "deferred_len_result_rejects_conflicting_annotation_before_lowering",
    "independent_causal_sites_survive_deferred_resolution",
    "deferred_projection_and_constructor_programs_finish_with_causal_results",
    "contaminated_base_keeps_independent_selector_type_error",
)


def main() -> None:
    population = json.loads((MUTATIONS / "population.json").read_text(encoding="utf-8"))
    valid = {item["id"] for item in population["mutations"] if not is_test_only(item)}
    rows = [json.loads(line) for line in (MUTATIONS / "results.jsonl").read_text(encoding="utf-8").splitlines()]
    assert len(rows) == len(valid) and {row["id"] for row in rows} == valid
    by_test = defaultdict(list)
    for row in rows:
        if row["result"]["release"] != "detectada":
            continue
        witness = row["witness"].get("release", "")
        for test in TESTS:
            if witness.endswith(test):
                by_test[test].append(row["id"])
                break
    lines = [START, "| Prueba que detecta | Sustituciones de producción |", "|---|---|"]
    for test in TESTS:
        if by_test[test]:
            lines.append(f"| `{test}` | {', '.join(sorted(by_test[test]))} |")
    lines.append(END)
    text = NOTES.read_text(encoding="utf-8")
    assert text.count(START) == text.count(END) == 1
    before, rest = text.split(START, 1)
    _, after = rest.split(END, 1)
    NOTES.write_text(before + "\n".join(lines) + after, encoding="utf-8", newline="\n")
    print(sum(map(len, by_test.values())))


if __name__ == "__main__":
    main()
