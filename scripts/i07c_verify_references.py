#!/usr/bin/env python3
"""Verifica nombres citados en notas, argumentos y resultados vigentes."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True


ROOT = Path(__file__).resolve().parents[1]
NOTES = ROOT / "docs/validation/F0-0002-I07c-implementation-notes.md"
ARGUMENTS = ROOT / "docs/validation/i07c-mutations/equivalence.json"
I10A_ARGUMENTS = ROOT / "docs/validation/i07c-mutations/i10a.json"
RESULTS = ROOT / "docs/validation/i07c-mutations/results.jsonl"
POPULATION = ROOT / "docs/validation/i07c-mutations/population.json"
SHORT_FUNCTIONS = {"bind", "branch", "expr", "function", "root", "union", "unify"}
NON_FUNCTION_IDENTIFIERS = {
    "array_child", "array_origin", "array_type_depth", "budget_scope",
    "concrete_intrinsic", "context_rejections", "copyable_data",
    "copyable_origin", "deferred_element", "deferred_exprs", "divergent_exprs", "element_expected",
    "emitted_sites", "expected_type", "failed_constructors", "failed_join_exprs",
    "found_type", "has_pending_projection", "incomplete_analysis", "index_tainted", "internal_error",
    "left_origin", "not_indexable", "partial_normal_exprs", "pending_scope",
    "source_sha256", "type_mismatch", "type_nodes", "type_solver_steps",
    "value_type_depth", "head_acceptance", "semantic_scale_counter", "is_some",
    "print_dv_array", "constructor_unresolved", "indexed_write_dv_array",
    "assign_divergent", "len_divergent", "assert_empty", "if_divergent",
    "index_divergent", "index_if_divergent", "less_divergent", "annot_divergent",
    "array_divergent", "equivalente_test", "cfg_test", "i07c", "i10a",
    # Named source forms, campaigns, and integration binaries are not functions.
    "array_branch_group", "array_independent_elements", "block_value_nested_branch",
    "group_nested_continuation", "binary_operand_and_later_assignment",
    "deferred_selector_condition", "deferred_release", "i07c_normative_survivors",
    # The migration ledger intentionally names the deleted I07a tests.
    "adr_gap_deferred_binding_takes_first_concrete_assignment_type",
    "adr_gap_deferred_var_aliases_do_not_unify_types",
    "adr_gap_never_is_bottom_for_equality_and_array_inference",
    "array_of_deferred_reads_infers_concrete_siblings_or_remains_bottom",
    "branch_union_rechecks_value_type_when_semantic_gate_is_bypassed",
    "cfg_lowering_requires_bool_without_checker",
    "numeric_lowering_expects_branch_value_at_its_own_span",
    "numeric_lowering_rejects_non_numeric_left_without_checker",
    "repeated_while_preheaders_have_linear_verification_time",
    "wholly_deferred_array_keeps_its_contextual_bottom_type",
}


def defined_names() -> set[str]:
    names = set()
    for folder in (ROOT / "src", ROOT / "tests", ROOT / "scripts"):
        for path in folder.rglob("*"):
            if path.suffix not in (".rs", ".py"):
                continue
            source = path.read_text(encoding="utf-8")
            names.update(re.findall(r"\b(?:fn|def)\s+([A-Za-z_][A-Za-z_0-9]*)\s*\(", source))
            if path.name == "i07c_deferred_mutation_cases.rs":
                names.update(re.findall(r"\bmutant!\s*\(\s*([A-Za-z_][A-Za-z_0-9]*)\s*,", source))
    return names


def main() -> None:
    from i07c_repair_text import verify

    checked_texts = verify()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--notes-only", action="store_true")
    args = parser.parse_args()
    notes = NOTES.read_text(encoding="utf-8")
    arguments = {} if args.notes_only else json.loads(ARGUMENTS.read_text(encoding="utf-8"))
    if not args.notes_only:
        arguments.update({f"i10a:{key}": value for key, value in
                          json.loads(I10A_ARGUMENTS.read_text(encoding="utf-8")).items()})
    names = defined_names()
    cited = {
        content for content in re.findall(r"`([^`]+)`", notes)
        if re.fullmatch(r"[a-z][a-z0-9_]*", content)
        and ("_" in content or content in SHORT_FUNCTIONS)
        and content not in NON_FUNCTION_IDENTIFIERS
    }
    missing = sorted(cited - names)
    assert not missing, f"nombres de función o prueba ausentes en notas: {missing}"
    for mutant, argument in arguments.items():
        assert not re.search(r"\b(?:src|tests)/[^\s),;]+\.rs:\d+|\b:\d+(?:-\d+)?\b", argument), (
            mutant, "referencia a línea mutable"
        )
        assert not re.search(r"SHA-256\s+[0-9a-f]{64}", argument), (
            mutant, "hash de archivo mutable"
        )
        for path_name in re.findall(r"\btests/[\w/-]+\.rs\b", argument):
            assert (ROOT / path_name).is_file(), (mutant, path_name)
        called = {
            name for name in re.findall(r"\b([a-z][a-z_0-9]*)\(", argument)
            if ("_" in name or name in SHORT_FUNCTIONS)
            and name not in NON_FUNCTION_IDENTIFIERS
        }
        quoted = {
            name for name in re.findall(r"`([a-z][a-z_0-9]*)`", argument)
            if ("_" in name or name in SHORT_FUNCTIONS)
            and name not in NON_FUNCTION_IDENTIFIERS
        }
        assert not ((called | quoted) - names), (
            mutant, "función o test ausente", sorted((called | quoted) - names)
        )
    if not args.notes_only:
        from i07c_mutaciones import is_test_only

        population = json.loads(POPULATION.read_text(encoding="utf-8"))["mutations"]
        assert all(not is_test_only(item) for item in population)
        for row in (json.loads(line) for line in RESULTS.read_text(encoding="utf-8").splitlines()):
            for witness in row.get("witness", {}).values():
                match = re.search(r"(?:^|::)([a-z][a-z_0-9]*)$", witness)
                if match:
                    assert match.group(1) in names, (row["id"], witness)
    print(f"Referencias comprobadas: {len(cited)} nombres y {len(arguments)} argumentos; {checked_texts} JSON/MD sin mojibake")


if __name__ == "__main__":
    main()
