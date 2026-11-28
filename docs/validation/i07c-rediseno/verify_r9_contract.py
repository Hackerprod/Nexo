#!/usr/bin/env python3
"""Check R6 ownership, C1 reference, D7 checker and immutable v11 blobs."""

import hashlib
import importlib.util
import json
import statistics
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
assert gates["schema"] == "i07c-r19-commit-gates-v9"
assert gates["frozen_v11_commit"] == "fcf62d5e43c6a29981abe478dd039db51b3cb524"
for relative, expected_sha in gates["frozen_v11_sha256"].items():
    blob = subprocess.run(
        ["git", "show", f"{gates['frozen_v11_commit']}:{relative}"], cwd=HERE.parents[2],
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
for family, owner_key in (
    ("directed74_normative_ids", "owner_directed"),
    ("matrix45_normative_ids", "owner_matrix"),
    ("frozen13_normative_ids", "owner_frozen13"),
):
    owners = gates[owner_key]
    for number in range(7):
        expected = {id_ for id_, at in owners.items() if int(at[1]) <= number}
        assert set(gates["commits"][f"C{number}"][family]) == expected, (family, number)
assert gates["directed74_unique_sources"] == 48
assert gates["commits"]["C1"]["c1_independent_probes"] == [
    "recursive_69", "recursive_78_equivalent", "recursive_127_equivalent",
]
assert gates["c1_reference_commit"] == "42cb63fa3c80abe551155f4c9ebd044f3e30914d"
reference = gates["c1_reference_evidence"]
assert subprocess.run(
    ["git", "rev-parse", gates["c1_reference_commit"] + "^{tree}"],
    cwd=HERE.parents[2], check=True, capture_output=True, text=True,
).stdout.strip() == reference["tree"]
for relative, field in (
    ("src/check/types.rs", "types_blob"),
    ("src/ir/i07c_tests.rs", "test_blob"),
    ("tests/i07c_normative_survivors.rs", "normative_test_blob"),
):
    assert subprocess.run(
        ["git", "rev-parse", f"{gates['c1_reference_commit']}:{relative}"],
        cwd=HERE.parents[2], check=True, capture_output=True, text=True,
    ).stdout.strip() == reference[field], relative
assert reference["production_commit"] == reference["parent_commit"]
assert reference["test_commit"] == gates["c1_reference_commit"]
assert gates["c1_reference_evidence"]["A_D_A_unmodeled_H_new"] == [0, 0, 0, 0]
assert gates["c1_reference_evidence"]["owned_C1_directed_matrix_frozen"] == [3, 0, 0]
assert gates["c1_reference_evidence"]["validate_sh"].startswith("GREEN:")
assert gates["c1_reference_evidence"]["parent_commit"] == subprocess.run(
    ["git", "rev-parse", gates["c1_reference_commit"] + "^"], cwd=HERE.parents[2],
    check=True, capture_output=True, text=True).stdout.strip()
assert gates["c1_previous_reference"]["commit"] == "1e4457fec7c02dc426afa74b25ae4660521c73a8"
c1_n = gates["c1_declared_main_difference"]
fixture = HERE.parents[2] / c1_n["fixture"]
assert len(fixture.read_bytes()) == c1_n["fixture_bytes"] == 100
assert hashlib.sha256(fixture.read_bytes()).hexdigest() == c1_n["fixture_sha256"]
assert c1_n["class"] == "N(b)(c)" and c1_n["owner"] == "C1"
assert c1_n["expected_adr_code_subcause_primary"] == [
    ["NX-E0102", "not_indexable", [57, 59]],
    ["NX-E0102", "type_mismatch", [81, 83]],
]
import d7_family_check as d7_check  # noqa: E402
assert hashlib.sha256((HERE / "d7_family_check.py").read_bytes()).hexdigest() == gates["d7_classifier_sha256"]
assert hashlib.sha256((HERE / "d7_cross_gate.py").read_bytes()).hexdigest() == gates["d7_cross_gate_sha256"]
assert d7_check.FROZEN == gates["frozen_v11_sha256"]["docs/validation/i07c-rediseno/candidate_gate.py"]
d7 = gates["d7_declared_main_differences"]
assert d7["owner"] == "C4" and len(d7["observed_unmodeled_residuals"]) == 3
witness = d7["manual_witness"]
assert len(witness["source"].encode("ascii")) == witness["bytes"] == 78
assert hashlib.sha256(witness["source"].encode("ascii")).hexdigest() == witness["source_sha256"]
assert witness["adr_code_primary"] == [["NX-E0103", [62, 64]], ["NX-E0103", [67, 70]]]
sites = list(d7_check.recursive_sites(witness["source"]))
assert len(sites) == 1 and d7_check.counterfactual(witness["source"], sites[0])[sites[0].start():sites[0].end()].isspace()
from audit_families import source  # noqa: E402
directed5 = data("dirigido5.json")
for index in (6, 214, 261, 335, 377, 543, 2069, 4425):
    assert list(d7_check.recursive_sites(source(directed5[index]))), index
for row_id, row in d7["observed_unmodeled_residuals"].items():
    corpus_id, index = row_id.split(":")
    assert corpus_id == "dirigido5" and row["profile"] == "f0-0002-b-v2"
    src = source(directed5[int(index)]).encode("ascii")
    assert len(src) == row["bytes"]
    assert hashlib.sha256(src).hexdigest() == row["source_sha256"]
assert all(gates["owner_directed"][ident] == "C4" for ident in
           ("atomic_78", "SB_equivalent", "W2_no_retroactive_context"))
assert all(gates["owner_matrix"][ident] == "C4" for ident in
           ("F06", "M2", "W0", "w1"))
assert gates["owner_matrix"]["W0nounion"] == "C3"
assert [len(gates["owned_corpus_ids"]["C2"][family]) for family in
        ("directed74", "matrix45", "frozen13")] == [10, 0, 3]
for owned_family, commit_family in (
    ("directed74", "directed74_normative_ids"),
    ("matrix45", "matrix45_normative_ids"),
    ("frozen13", "frozen13_normative_ids"),
):
    assert set(gates["owned_corpus_ids"]["C2"][owned_family]) == set(
        gates["commits"]["C2"][commit_family]
    ), owned_family
assert gates["owner_matrix"]["F06"] == "C4"
assert gates["commits"]["C1"]["f06_cross_gate"] == {
    "expected": "main_or_full_adr_oracle", "no_oracle_to_main": True,
}
assert [len(gates["commits"][f"C{i}"]["matrix45_normative_ids"]) for i in range(7)] == [
    0, 0, 0, 5, 13, 45, 45,
]
assert [len(gates["commits"][f"C{i}"]["directed74_normative_ids"]) for i in range(7)] == [
    0, 3, 10, 24, 32, 74, 74,
]
contract = (HERE.parents[1] / "correcciones/I07c-rediseno-restauracion-atomica-R6-final.md").read_text(encoding="utf-8")
c3_row = next(line for line in contract.splitlines() if line.startswith("| C3 |"))
assert "**24/74, 5/45, 9/13**" in c3_row
assert all(name not in c3_row for name in ("atomic_78", "F06", "W0/"))
readme = (HERE / "README.md").read_text(encoding="utf-8")
assert "F06 pertenece a C4" in readme and "F06 pertenece a C3" not in readme
assert all("F06" not in gates["commits"][f"C{i}"]["matrix45_normative_ids"] for i in range(3))
assert all("F06" in gates["commits"][f"C{i}"]["matrix45_normative_ids"] for i in range(4, 7))
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
assert baseline["classifier_sha256"] == gates["frozen_v11_sha256"]["docs/validation/i07c-rediseno/candidate_gate.py"]
import owned_rows_gate as owned  # noqa: E402
assert tuple(name for name, _ in owned.FAMILIES) == (
    "directed74_normative_ids", "matrix45_normative_ids", "frozen13_normative_ids")
for family, filename in owned.FAMILIES:
    available = {row["id"] for row in data(filename)["rows"]}
    assert set(gates["owner_" + {"directed74_normative_ids": "directed",
                                  "matrix45_normative_ids": "matrix",
                                  "frozen13_normative_ids": "frozen13"}[family]]) <= available
    for number in range(1, 7):
        assert set(gates["commits"][f"C{number}"][family]) <= available
assert owned.matrix_signature(next(row for row in data("r8-matrix45.json")["rows"] if row["id"] == "F06"))[0] == "rejected"
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
assert len(migration) == len({x["source_test"] for x in migration}) == 125
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
assert sum(bool(x.get("approval_required")) for x in migration) == 32
g3_migration = next(x for x in migration if x["source_test"] == "grouped_operands_keep_one_causal_primary")
assert g3_migration["assertion_before"].endswith("240 sites")
assert g3_migration["assertion_after"].endswith("246 sites")
assert g3_migration["approval_required"] and g3_migration["owner_commit"] == "C2"
assert g3_migration["approval_status"].startswith("aprobada por el coordinador")
quota_migration = next(x for x in migration if x["source_test"] ==
                       "i07c_survivor3_partial_union_allocation_rolls_back_on_quota")
assert "2026-11-26 06:37 PT" in quota_migration["approval"]
assert "C2 4e63e33" in quota_migration["approval"]
assert len(manifest["private_23"]) == len(set(manifest["private_23"])) == 23
assert all(next(x for x in migration if x["source_test"] == name)["private_checker_api"]
           for name in manifest["private_23"])
assert len(manifest["explicit_approval_31"]["existing_quota_and_precedence"]) == 17
assert len(manifest["explicit_approval_31"]["new_fixed_cost_quota"]) == 10
assert len(manifest["explicit_approval_31"]["formerly_preserved_fixed_cost_quota"]) == 4
approval = manifest["additional_approval_v15b"]
assert approval["authority"] == "Coordinador, 2026-11-26 20:30 PT"
assert [row["case"] for row in approval["expectations"]] == [
    "resource_array_write", "resource_projection",
]
assert [row["class"] for row in approval["expectations"]] == ["I", "N(b)(c)"]
assert "expected_adr_code_subcause_primary" not in approval["expectations"][0]
assert approval["expectations"][0]["expected_migrated_code_subcause_primary"] == [
    ["NX-E0102", "type_mismatch", [78, 80]],
]
assert approval["expectations"][1]["fixture_sha256"] == c1_n["fixture_sha256"]
assert approval["expectations"][1]["expected_adr_code_subcause_primary"] == c1_n["expected_adr_code_subcause_primary"]

c2 = gates["c2_reference_evidence_v15d"]
c2_commit = "184f2d5568fe52badbec20f8a9a2841566999931"
assert c2["base_commit"] == "af19a34eb295fdd4834bdbc4594e442fbc2b42ec"
assert subprocess.run(
    ["git", "rev-parse", c2_commit + "^{tree}"], cwd=HERE.parents[2],
    check=True, capture_output=True, text=True,
).stdout.strip() == c2["tree"]
for relative, field in (
    ("src/check/types.rs", "types_blob"),
    ("scripts/i07c_c0_span_audit.py", "span_audit_blob"),
    ("docs/validation/i07c-rediseno/c2-span-key-usos.json", "span_inventory_blob"),
    ("tests/i07c_structural_invariants.rs", "structural_test_blob"),
):
    blob = subprocess.run(
        ["git", "rev-parse", f"{c2_commit}:{relative}"], cwd=HERE.parents[2],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    assert blob == c2[field], relative
span_inventory = (HERE / "c2-span-key-usos.json").read_bytes().replace(b"\r\n", b"\n")
assert hashlib.sha256(span_inventory).hexdigest() == c2["span_inventory_sha256"]
for name, expected_sha in c2["report_sha256"].items():
    relative = f"docs/validation/i07c-rediseno/{name}"
    blob = subprocess.run(
        ["git", "show", f"HEAD:{relative}"], cwd=HERE.parents[2],
        check=True, capture_output=True,
    ).stdout
    assert hashlib.sha256(blob).hexdigest() == expected_sha, name
    worktree = (HERE / name).read_bytes().replace(b"\r\n", b"\n")
    assert hashlib.sha256(worktree).hexdigest() == expected_sha, name

c2_corpus = data("c2-ref-corpus-v15d.json")
assert c2_corpus["complete"] and c2_corpus["commit"] == "C2" and not c2_corpus["failures"]
assert len(c2_corpus["corpora"]) == c2["corpora_complete"] == 20
assert len(c2_corpus["class_by_row"]) == c2["rows_complete"] == 151188
from collections import Counter  # noqa: E402
assert dict(Counter(c2_corpus["class_by_row"].values())) == c2["classes"]
assert c2["A_D_A_unmodeled_H_divergent"] == [0, 0, 0, 0]
assert c2_corpus["class_by_row"] == data("r14-main-baseline.json")["class_by_row"]
assert c2_corpus["main_sha256"] == c2["main_release_sha256"]
assert c2_corpus["candidate_sha256"] == c2["candidate_release_sha256"]

c2_owned = data("c2-ref-owned-v15d.json")
assert c2_owned["complete"] and c2_owned["commit"] == "C2" and not c2_owned["failures"]
assert [c2_owned["counts"][key] for key in (
    "directed74_normative_ids", "matrix45_normative_ids", "frozen13_normative_ids",
)] == c2["owned_C2_directed_matrix_frozen"] == [10, 0, 3]
assert c2_owned["f06_class"] == c2["f06_class"] == "main"
assert c2_owned["candidate_sha256"] == c2["candidate_release_sha256"]

c2_disc = data("c2-ref-discriminators-v15d.json")
assert c2_disc["complete"] and not c2_disc["failures"]
assert len(c2_disc["cases"]) == c2["discriminator_profile_cases"] == 18
assert c2_disc["candidate_sha256"] == c2["candidate_release_sha256"]
for row in c2_disc["cases"].values():
    assert set(row["observed"]) == {"check", "run"}
    assert all(observed["diagnostics"] == row["expected"] for observed in row["observed"].values())
import c2_discriminator_gate as c2_gate  # noqa: E402
for extension, case in zip(c2["d_prime_extensions"],
                           ("resource_rejected_array", "repeated_recursive_array"), strict=True):
    source, expected = c2_gate.CASES[case]
    assert extension["source"] == source
    assert extension["bytes"] == len(source.encode("ascii"))
    assert extension["source_sha256"] == hashlib.sha256(source.encode("ascii")).hexdigest()
    assert extension["expected_code_subcause_primary"] == [
        [code, subcause, [start, end]] for code, subcause, start, end in expected
    ]

frame = (HERE / "c2-ref-frame-v15d.txt").read_text(encoding="utf-8")
assert f"pair={c2['frame_pair_bytes']} margin={c2['frame_margin_bytes']}" in frame
assert c2["frame_margin_bytes"] >= 1000
perf = data("c2-ref-perf-v15d.json")
assert perf["pairs"] == 4000 and perf["profile"] == "f0-0003-b-v1"
assert perf["binary_sha256"] == {"main": c2["main_release_sha256"],
                                 "C2": c2["candidate_release_sha256"]}
ratio = statistics.median(perf["seconds"]["C2"]) / statistics.median(perf["seconds"]["main"])
assert ratio == perf["median_ratio"] == c2["perf_median_ratio"] and ratio <= 3
assert c2["validate_sh"].startswith("GREEN: debug 2230/2230, release 2232/2232")

import candidate_gate as cross_gate  # noqa: E402

adjudication = data("c2-d-adjudicacion-v15d.json")
assert len(adjudication["rows"]) == 381
assert adjudication["candidate_sha256"] == c2["candidate_debug_sha256"]
assert adjudication["candidate_release_sha256"] == c2["candidate_release_sha256"]
assert adjudication["debug_release_equal_rows"] == 381
assert adjudication["main_sha256"] == c2["main_release_sha256"]
assert adjudication["main_equal_rows"] == 381
from collections import Counter  # noqa: E402
assert dict(Counter(row["family"] for row in adjudication["rows"])) == adjudication["counts"]
for index, row in enumerate(adjudication["rows"]):
    assert row["probe_index"] == index and row["profile"] == "f0-0003-b-v1"
    assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"]
    assert cross_gate.oracle_signature(oracle.derive(row["source"]))[1] == row["oracle"]
    if row["new_class"] == "D":
        assert "(if " in row["source"]
        assert not any(item[5] and item[5][0] > item[2][0] for item in row["candidate"])
assert [sum(row["new_class"] == cls for row in adjudication["rows"])
        for cls in ("B", "C", "D")] == [6, 1, 374]

sequence = data("c3-secuencia-v15d-filas.json")
assert len(sequence["rows"]) == 45
assert sequence["candidate_sha256"] == c2["candidate_debug_sha256"]
assert sum(row["c2_exact"] for row in sequence["rows"]) == 30
assert [sum(row["commit"] == commit for row in sequence["rows"])
        for commit in ("C3", "C4")] == [38, 7]
owned_c3 = data("c3-secuencia-v15d-owned.json")
assert owned_c3["complete"] and owned_c3["commit"] == "C3"
assert owned_c3["candidate_sha256"] == c2["candidate_debug_sha256"]
assert owned_c3["main_sha256"] == "efc801d2f76b8ae972d1133ab9118b67db5f4bfacaa4d0873fdd4c3dc021e40a"
assert [owned_c3["counts"][family] for family, _ in owned.FAMILIES] == [24, 5, 9]
assert len(owned_c3["failures"]) == 8
assert {failure.split(":", 2)[1] for failure in owned_c3["failures"]} == {
    "M2_partial_witness_chain", "RS1_condition_contaminated",
    "W1_atomic_compound", "W1a_without_later_assignment",
}
for row in sequence["rows"]:
    family = row["family"]
    assert row["id"] in gates["commits"][row["commit"]][family]
    owner_map = gates[{"directed74_normative_ids": "owner_directed",
                       "matrix45_normative_ids": "owner_matrix",
                       "frozen13_normative_ids": "owner_frozen13"}[family]]
    assert owner_map[row["id"]] <= row["commit"]
named = data("c3-secuencia-v15d-named.json")
assert named["candidate_sha256"] == c2["candidate_debug_sha256"]
assert [row["id"] for row in named["rows"]] == gates["commits"]["C3"]["additional_named_probes"]
assert sum(row.get("c2_main_exact", False) for row in named["rows"]) == 7
assert [sum(row["kind"] == kind for row in named["rows"])
        for kind in ("duplicate_owned_row", "aggregate", "source")] == [3, 1, 7]

# v15e is the active C2 reference; v15d above remains auditable history.
c2e = gates["c2_reference_evidence"]
c2e_commit = gates["c2_reference_commit"]
assert c2e_commit == c2e["code_commit"] == "be96e08f1a18fa92e296fe89d1dc78dfd805a55f"
assert subprocess.run(
    ["git", "rev-parse", c2e_commit + "^{tree}"], cwd=HERE.parents[2],
    check=True, capture_output=True, text=True,
).stdout.strip() == c2e["tree"]
for relative, field in (
    ("src/check/types.rs", "types_blob"),
    ("tests/i07c_structural_invariants.rs", "structural_test_blob"),
):
    assert subprocess.run(
        ["git", "rev-parse", f"{c2e_commit}:{relative}"], cwd=HERE.parents[2],
        check=True, capture_output=True, text=True,
    ).stdout.strip() == c2e[field]
for name, expected_sha in c2e["report_sha256"].items():
    relative = f"docs/validation/i07c-rediseno/{name}"
    revision = "02de85f" if name == "c2_literal_origin_fuzz.py" else "HEAD"
    blob = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=HERE.parents[2],
        check=True, capture_output=True,
    ).stdout
    assert hashlib.sha256(blob).hexdigest() == expected_sha, name
    if revision == "HEAD":
        assert hashlib.sha256((HERE / name).read_bytes().replace(b"\r\n", b"\n")).hexdigest() == expected_sha, name
corpus_e = data("c2-ref-corpus-v15e.json")
assert corpus_e["complete"] and corpus_e["commit"] == "C2" and not corpus_e["failures"]
assert len(corpus_e["corpora"]) == 20 and len(corpus_e["class_by_row"]) == 151188
assert corpus_e["class_by_row"] == data("r14-main-baseline.json")["class_by_row"]
assert corpus_e["candidate_sha256"] == c2e["candidate_release_sha256"]
assert corpus_e["main_sha256"] == c2e["main_release_sha256"]
assert dict(Counter(corpus_e["class_by_row"].values())) == c2e["classes"]
assert c2e["candidate_gate_debug"] == "NO EVALUADO"
for name, candidate_sha in (("c2-ref-owned-v15e.json", c2e["candidate_release_sha256"]),
                            ("c2-ref-owned-debug-v15e.json", c2e["candidate_debug_sha256"])):
    report = data(name)
    assert report["complete"] and not report["failures"] and report["commit"] == "C2"
    assert report["candidate_sha256"] == candidate_sha and report["f06_class"] == "main"
    assert report["main_sha256"] == (c2e["main_debug_sha256"] if "debug" in name
                                     else c2e["main_release_sha256"])
    assert [report["counts"][family] for family, _ in owned.FAMILIES] == [10, 0, 3]
historical_rel_cases = {
    "first_literal_operand_after_completion",
    "first_operand_of_second_literal",
    "first_literal_operand_with_index_variable",
}
historical_discriminator_sha = hashlib.sha256(subprocess.run(
    ["git", "show", "02de85f:docs/validation/i07c-rediseno/c2_discriminator_gate.py"],
    cwd=HERE.parents[2], check=True, capture_output=True,
).stdout).hexdigest()
for name, candidate_sha in (("c2-ref-discriminators-v15e.json", c2e["candidate_release_sha256"]),
                            ("c2-ref-discriminators-debug-v15e.json", c2e["candidate_debug_sha256"])):
    report = data(name)
    assert report["complete"] and not report["failures"] and len(report["cases"]) == 24
    assert report["candidate_sha256"] == candidate_sha
    assert report["script_sha256"] == historical_discriminator_sha
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1"):
        for case in historical_rel_cases:
            spans = c2_gate.EXPECTED_REL[case]
            row = report["cases"][f"{profile}:{case}"]
            for verb in ("check", "run"):
                assert all(row["observed"][verb]["expected_type_spans"][i] == list(span)
                           for i, span in spans.items())
before_e = data("c2-ref-regression-before-v15e.json")
assert before_e["candidate_sha256"] == c2["candidate_release_sha256"]
assert len(before_e["failures"]) == 12
assert {failure.split(":")[1] for failure in before_e["failures"]} == historical_rel_cases
fuzz_e = data("c2-ref-fuzz-v15e.json")
assert fuzz_e["complete"] and fuzz_e["programs"] == 14000 and not fuzz_e["failures"]
assert fuzz_e["candidate_sha256"] == c2e["candidate_release_sha256"]
assert fuzz_e["counts"].get("A", 0) == fuzz_e["counts"].get("error", 0) == 0
historical_fuzz_sha = hashlib.sha256(subprocess.run(
    ["git", "show", "02de85f:docs/validation/i07c-rediseno/c2_literal_origin_fuzz.py"],
    cwd=HERE.parents[2], check=True, capture_output=True,
).stdout).hexdigest()
assert fuzz_e["script_sha256"] == historical_fuzz_sha
frame_e = (HERE / "c2-ref-frame-v15e.txt").read_text(encoding="utf-8")
assert frame_e.splitlines()[0] == (f"executable_sha256={c2e['frame_exe_sha256']} "
                                   f"map_sha256={c2e['frame_map_sha256']}")
assert f"pair={c2e['frame_pair_bytes']} margin={c2e['frame_margin_bytes']}" in frame_e
assert c2e["frame_margin_bytes"] >= 1000
perf_e = data("c2-ref-perf-v15e.json")
assert perf_e["binary_sha256"] == {"main": c2e["main_release_sha256"],
                                   "C2": c2e["candidate_release_sha256"]}
assert perf_e["median_ratio"] == c2e["perf_median_ratio"] <= 3
c3_owned_e = data("c3-secuencia-v15e-owned.json")
assert c3_owned_e["commit"] == "C3" and c3_owned_e["candidate_sha256"] == c2e["candidate_release_sha256"]
assert [c3_owned_e["counts"][family] for family, _ in owned.FAMILIES] == [24, 5, 9]
assert len(c3_owned_e["failures"]) == 8
validate_e = data("c2-ref-validate-v15e.json")
assert validate_e["code_commit"] == c2e_commit and validate_e["clean_clone"]
assert validate_e["rustc"].startswith("rustc 1.99.1 ") and validate_e["jobs"] == 4
assert validate_e["nextest_test_threads"] == 4 and validate_e["job_limit_inside_lock"]
assert validate_e["runner_profile"] == "default"
assert validate_e["mechanical"] == {"fmt": "passed", "clippy_deny_warnings": "passed",
                                     "nextest_selection": "passed"}
assert validate_e["debug"] == {"run": 2230, "passed": 2230, "skipped": 73}
assert validate_e["release"] == {"run": 2232, "passed": 2232, "skipped": 74}
assert validate_e["stack65"] == {"option_match126": "5/5", "result_match126": "5/5"}
assert validate_e["g3_auditor"] == {"sites": 246, "self_test": "passed"}
assert validate_e["log_sha256"] == c2e["validate_log_sha256"]
assert c2e["validate_sh"].startswith("GREEN: debug 2230/2230, release 2232/2232")
print("R6 contract structure OK")
