#!/usr/bin/env python3
"""Check the structural integrity of the R6 v9 frozen specification."""

import hashlib
import importlib.util
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent


def data(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


for name in (
    "r9-witness-discriminator.json", "r9-f2-variants.json", "r8-m1.json",
    "r8-match-main.json", "r8-f2-ajeno4.json", "r10-f2-alias.json", "r8-directed74.json",
    "r8-matrix45.json", "r8-t5-array5.json",
):
    rows = data(name)["rows"]
    for row in rows:
        actual = hashlib.sha256(row["source"].encode("ascii")).hexdigest()
        assert actual == row["source_sha256"], (name, row.get("id"))
    print(name, len(rows))

oracle_path = HERE.parents[2] / "scripts/i07c_rd_oracle.py"
spec = importlib.util.spec_from_file_location("i07c_rd_oracle", oracle_path)
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
for row in data("r8-matrix45.json")["rows"]:
    assert oracle.derive(row["source"])["diagnostics"] == row["expected"], row["id"]
print("matrix oracle", 45)

gates = data("r9-commit-gates.json")
for relative, expected_sha in gates["frozen_v8_sha256"].items():
    blob = subprocess.run(
        ["git", "show", f"HEAD:{relative}"], cwd=HERE.parents[2],
        check=True, capture_output=True,
    ).stdout
    actual_sha = hashlib.sha256(blob).hexdigest()
    assert actual_sha == expected_sha, relative
for family, size in (
    ("directed74_normative_ids", 74), ("matrix45_normative_ids", 45),
    ("frozen13_normative_ids", 13),
):
    previous = set()
    for number in range(7):
        current = set(gates["commits"][f"C{number}"][family])
        assert previous <= current
        previous = current
    assert len(previous) == size
assert gates["directed74_unique_sources"] == 48
assert gates["commits"]["C1"]["c1_independent_probes"] == [
    "atomic_78", "recursive_69", "recursive_78_equivalent",
    "recursive_127_equivalent", "F06", "W0", "W0nounion",
]
assert not gates["commits"]["C1"]["w1_w1a_full_normative"]
assert gates["commits"]["C3"]["w1_w1a_full_normative"]
assert gates["commits"]["C3"]["match_observations_29_expected_by_authority"]
assert all(len(gates["owned_corpus_ids"][f"C{i}"]["match_rows"]) == 103 for i in range(7))
assert all(gates["commits"][f"C{i}"]["match103_main_conservation"] for i in range(3))
assert all(gates["commits"][f"C{i}"]["match88_main_exact"] and
           gates["commits"][f"C{i}"]["match15_d6_adr_by_row"] and
           gates["commits"][f"C{i}"]["match_exterior7_main_exact"] for i in range(3, 7))
assert all(gates["commits"][f"C{i}"]["f2_ne_if_completion_then_assignment"] for i in range(2, 7))
assert len(gates["owned_corpus_ids"]["C3"]["match_extra_main"]) == 27
assert len(gates["owned_corpus_ids"]["C3"]["match_extra_adr"]) == 2
corpus_ids = [item["id"] for item in data("r7-frozen-batteries.json")["corpora"]]
assert len(corpus_ids) == 20
assert gates["owned_corpus_ids"]["C6"]["all_modelable_corpus_ids"] == corpus_ids
assert all(not gates["owned_corpus_ids"][f"C{i}"]["all_modelable_corpus_ids"] for i in range(6))

witnesses = data("r9-witness-discriminator.json")["rows"]
assert len(witnesses) == len({row["id"] for row in witnesses}) == 29
matrix_match = data("r8-match-main.json")["rows"]
d6 = data("r13-match-d6.json")["rows"]
assert [row["index"] for row in d6] == [5, 7, 37, 57, 59, 62, 65, 67, 68, 69, 71, 72, 78, 79, 102]
for row in d6:
    old = matrix_match[row["index"]]
    assert row["source"] == old["source"] and row["expected_main"] == old["expected_main"]
    assert row["proposed_r6_full_match"] != row["expected_main"]
    assert hashlib.sha256(row["standalone_if_source"].encode("ascii")).hexdigest() == row["standalone_if_source_sha256"]
    assert oracle.derive(row["standalone_if_source"])["diagnostics"] == row["standalone_if_adr"]
exterior = data("r13-match-exterior.json")["rows"]
assert len(exterior) >= 6
for row in exterior:
    assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"]
    assert row["if_value_events_with_expected"]
    assert all(event["expected_in_branch_accept"] == 1 for event in row["if_value_events_with_expected"])
ne = data("r13-f2-ne.json")["rows"][0]
assert oracle.derive(ne["source"])["diagnostics"] == ne["expected_adr"]["diagnostics"]
assert ne["expected_adr"]["diagnostics"][0]["rel_expected"] == [65, 66]
family = data("r13-family-audit.json")
assert sum(family["corpora"]["c1"]["counts"].values()) == 24000
assert family["corpora"]["c1"]["counts"]["provenance_relation"] == 524
assert family["corpora"]["c1"]["counts"]["equal"] == 20649
hard = data("r14-main-hard.json")
assert hard["repair_commit"] == "C6"
assert set(hard["rows"]) == {"s7021993:2024", "s9908255:2902"}
for row in hard["rows"].values():
    assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"]
    assert row["main_observed"] == {"exit": 2, "outcome": {"kind": "driver_error", "reason": "internal_error"}, "diagnostics": []}
    assert row["expected_adr_signature"] == ["checked", []]
baseline = data("r14-main-baseline.json")
assert baseline["complete"] and not baseline["failures"]
assert len(baseline["corpora"]) == 20 and len(baseline["class_by_row"]) == 151188
assert {key for key, label in baseline["class_by_row"].items() if label == "H"} == set(hard["rows"])
assert baseline["classifier_sha256"] == gates["frozen_v8_sha256"]["docs/validation/i07c-rediseno/candidate_gate.py"]
for index in (4, 20, 44, 46):
    probe = next(row for row in witnesses if row["id"] == f"match-index-{index}")
    assert probe["expected_main"] == matrix_match[index]["expected_main"]
    assert probe["source"] == matrix_match[index]["source"]

f2_variants = data("r9-f2-variants.json")["rows"]
assert len(f2_variants) == 9
for id_ in ("result_ok_before_err", "result_two_mismatches"):
    row = next(r for r in f2_variants if r["id"] == id_)
    assert row["authority"] == "class_I_revertible"
    assert "expected_adr" not in row
for id_ in ("P3_result_both_payloads_pending", "P3_result_both_payloads_pending_reverse_completion", "P4_result_err_only_pending"):
    row = next(r for r in f2_variants if r["id"] == id_)
    assert row["source"][77:83] == "Err(r)"
    assert row["expected_adr"]["diagnosticos"][0]["relations"][0]["span"] == [77, 83]
p5 = next(r for r in f2_variants if r["id"] == "P5_result_only_ok_mismatch")
assert p5["source"][66:71] == "Ok(q)"
assert p5["expected_adr"]["diagnosticos"][0]["relations"][0]["span"] == [66, 71]
assert p5["difference_main"]["main_span"] == [77, 83]
for row in data("r10-f2-alias.json")["rows"]:
    got = oracle.derive(row["source"])["diagnostics"]
    assert got[0]["rel_expected"] == [16, 19], row["id"]

f2 = next(x for x in data("r8-f2-ajeno4.json")["rows"] if x["id"] == "c1-13262")
assert f2["expected_main_compact"][0][-1] == [142, 143]
assert f2["expected_code_subcause_primary_types_relation"][0][-1] == [90, 93]

manifest = data("r9-test-migration.json")
migration = manifest["rows"]
assert len(migration) == len({x["source_test"] for x in migration}) == 124
assert sum(x["class"] == "former_65" for x in migration) == 65
assert sum(x["class"] == "former_65" and x["private_checker_api"] for x in migration) == 44
for action, count in (
    ("conservar_sin_cambio", 7), ("sustituir_mecanismo_por_cli", 8),
    ("reescribir_precedencia_recorrido_comprobacion", 5),
    ("reescribir_cuota_propiedad_11_1", 26),
    ("portar_observable_cli_y_privado_libro", 63),
    ("portar_invariante_privada_modelo_nuevo", 15),
):
    assert sum(x["action"] == action for x in migration) == count
assert sum(bool(x.get("approval_required")) for x in migration) == 31
assert len(manifest["private_23"]) == len(set(manifest["private_23"])) == 23
assert all(next(x for x in migration if x["source_test"] == name)["private_checker_api"]
           for name in manifest["private_23"])
assert len(manifest["explicit_approval_31"]["existing_quota_and_precedence"]) == 17
assert len(manifest["explicit_approval_31"]["new_fixed_cost_quota"]) == 10
assert len(manifest["explicit_approval_31"]["formerly_preserved_fixed_cost_quota"]) == 4
print("r10 contract structure OK")
