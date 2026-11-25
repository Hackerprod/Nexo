#!/usr/bin/env python3
"""Freeze the growing, named fixture subsets for C0–C6."""

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def rows(name, key="rows"):
    return json.loads((HERE / name).read_text(encoding="utf-8"))[key]


directed = rows("r8-directed74.json")
assert len(directed) == 74
assert len({r["source_sha256"] for r in directed}) == 48
matrix = rows("r8-matrix45.json")
assert len(matrix) == 45
owner_directed = {r["id"]: "C5" for r in directed}
for name in ("atomic_78", "recursive_69", "recursive_78_equivalent", "recursive_127_equivalent"):
    owner_directed[name] = "C1"
for name in ("B_M1", "B_M2", "planned", "ST1_equivalent", "RC1_read_context_variable", "RC2_read_context_committed", "RC3_read_context_no_retroactive"):
    owner_directed[name] = "C2"
for name in ("SB_equivalent", "join_record", "W1_atomic_compound", "W1a_without_later_assignment", "M2_partial_witness_chain", "RS1_condition_contaminated", "RS2_independent_child_errors", "RS3_partial_in_array", "W2_no_retroactive_context"):
    owner_directed[name] = "C3"
for name in list(owner_directed):
    if name.startswith("X_0"):
        owner_directed[name] = "C3"
for name in ("T5_constructor_equivalent", "S_A_independent_constructor_equiv", "CD1_independent_constructor", "CD2_independent_error_and_duties", "completion_unresolved"):
    owner_directed[name] = "C4"
owner_matrix = {r["id"]: "C5" for r in matrix}
for name in ("W0nounion", "W0", "F06"):
    owner_matrix[name] = "C1"
for name in ("R1", "R1twin", "R2", "R3", "M2", "w1"):
    owner_matrix[name] = "C3"
for name in ("O01", "O03", "O07", "O08"):
    owner_matrix[name] = "C4"
assert set(owner_directed) == {r["id"] for r in directed}
assert set(owner_matrix) == {r["id"] for r in matrix}

probes = rows("r7-frozen-batteries.json", "probes")
owner_probe = {r["id"]: "C5" for r in probes}
for name in ("RC1_read_context_variable", "RC2_read_context_committed", "RC3_read_context_no_retroactive"):
    owner_probe[name] = "C2"
for name in ("W1_atomic_compound", "W1a_without_later_assignment", "M2_partial_witness_chain", "RS1_condition_contaminated", "RS2_independent_child_errors", "RS3_partial_in_array"):
    owner_probe[name] = "C3"
for name in ("CD1_independent_constructor", "CD2_independent_error_and_duties"):
    owner_probe[name] = "C4"

commits = {}
for number in range(7):
    commit = f"C{number}"
    selected = lambda owners: sorted(name for name, at in owners.items() if int(at[1]) <= number)
    commits[commit] = {
        "directed74_normative_ids": selected(owner_directed),
        "matrix45_normative_ids": selected(owner_matrix),
        "frozen13_normative_ids": selected(owner_probe),
        "f2_22_normative": number >= 2,
        "f2_variant5_normative": number >= 2,
        "m1_2_normative": number >= 3,
        "match103_main_conservation": number >= 0,
        "t5_array5_normative": number >= 3,
        "all_modeled_corpus_normative_full": number >= 6,
        "other_fixtures": "main exact except an ADR-declared difference already owned by this commit",
        "c0_main_exact_all_20_corpus_profiles": number == 0,
        "w1_w1a_c1_partial_code_primary": [["NX-E0102", 99, 100], ["NX-E1303", 147, 148], ["NX-E1303", 199, 200], ["NX-E1303", 209, 212]] if number == 1 else None,
    }
    print(commit, len(commits[commit]["directed74_normative_ids"]), len(commits[commit]["matrix45_normative_ids"]), len(commits[commit]["frozen13_normative_ids"]))
assert len(commits["C6"]["directed74_normative_ids"]) == 74
assert len(commits["C6"]["matrix45_normative_ids"]) == 45
assert len(commits["C6"]["frozen13_normative_ids"]) == 13
out = {"schema": "i07c-r9-commit-gates-v1", "base_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
       "directed74_unique_sources": 48, "directed74_duplicate_rows": 26,
       "owner_directed": owner_directed, "owner_matrix": owner_matrix, "owner_frozen13": owner_probe,
       "commits": commits}
(HERE / "r9-commit-gates.json").write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
