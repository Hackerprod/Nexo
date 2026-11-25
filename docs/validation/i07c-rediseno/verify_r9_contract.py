#!/usr/bin/env python3
"""Check the structural integrity of the R6 v4 frozen specification."""

import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def data(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


for name in ("r9-witness-discriminator.json", "r9-f2-variants.json", "r8-m1.json", "r8-match-main.json", "r8-f2-ajeno4.json", "r8-directed74.json", "r8-matrix45.json", "r8-t5-array5.json"):
    rows = data(name)["rows"]
    for row in rows:
        assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"], (name, row.get("id"))
    print(name, len(rows))

oracle_path = HERE.parents[2] / "scripts/i07c_rd_oracle.py"
spec = importlib.util.spec_from_file_location("i07c_rd_oracle", oracle_path)
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
for row in data("r8-matrix45.json")["rows"]:
    assert oracle.derive(row["source"])["diagnostics"] == row["expected"], row["id"]
print("matrix oracle", 45)

gates = data("r9-commit-gates.json")
for family, size in (("directed74_normative_ids", 74), ("matrix45_normative_ids", 45), ("frozen13_normative_ids", 13)):
    previous = set()
    for number in range(7):
        current = set(gates["commits"][f"C{number}"][family])
        assert previous <= current
        previous = current
    assert len(previous) == size
assert gates["directed74_unique_sources"] == 48
assert gates["commits"]["C1"]["c1_excluded_until_c3"]
assert gates["commits"]["C1"]["c1_independent_probes"] == [
    "atomic_78", "recursive_69", "recursive_78_equivalent",
    "recursive_127_equivalent", "F06", "W0", "W0nounion",
]
assert not gates["commits"]["C1"]["w1_w1a_full_normative"]
assert gates["commits"]["C3"]["w1_w1a_full_normative"]
assert gates["commits"]["C3"]["match_conservation_24_full_main"]

witnesses = data("r9-witness-discriminator.json")["rows"]
assert len(witnesses) == 24
assert len({row["id"] for row in witnesses}) == 24
matrix_match = data("r8-match-main.json")["rows"]
for index in (4, 44):
    probe = next(row for row in witnesses if row["id"] == f"match-index-{index}")
    assert probe["expected_main"] == matrix_match[index]["expected_main"]
    assert probe["source"] == matrix_match[index]["source"]

f2_variants = data("r9-f2-variants.json")["rows"]
assert len(f2_variants) == 7
for id_ in ("result_ok_before_err", "result_two_mismatches"):
    row = next(r for r in f2_variants if r["id"] == id_)
    assert row["authority"] == "class_I_revertible"
    assert "expected_adr" not in row
    assert row["expected_class_I_conservation"]
for id_ in ("P3_result_both_payloads_pending", "P4_result_err_only_pending"):
    row = next(r for r in f2_variants if r["id"] == id_)
    assert row["source"][77:83] == "Err(r)"
    assert row["expected_adr"]["diagnosticos"][0]["relations"][0]["span"] == [77, 83]

f2 = next(x for x in data("r8-f2-ajeno4.json")["rows"] if x["id"] == "c1-13262")
assert f2["expected_main_compact"][0][-1] == [142, 143]
assert f2["expected_code_subcause_primary_types_relation"][0][-1] == [90, 93]

migration = data("r9-test-migration.json")["rows"]
assert len(migration) == len({x["source_test"] for x in migration}) == 124
assert sum(x["class"] == "former_65" for x in migration) == 65
assert sum(x["class"] == "former_65" and x["private_checker_api"] for x in migration) == 21
assert sum(x["action"] == "conservar_sin_cambio" for x in migration) == 40
assert sum(x["action"] == "sustituir_mecanismo_por_cli" for x in migration) == 8
assert sum(x["action"] == "reescribir_precedencia_recorrido_comprobacion" for x in migration) == 5
assert sum(x["action"] == "reescribir_cuota_propiedad_11_1" for x in migration) == 12
assert all(x.get("private_invariant_new_model") for x in migration if x["action"] == "portar_observable_cli_y_privado_libro")
assert all(x.get("approval", "").startswith("aprobación expresa") for x in migration if x["action"] in ("reescribir_precedencia_recorrido_comprobacion", "reescribir_cuota_propiedad_11_1"))

print("r9 contract structure OK")
