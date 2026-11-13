#!/usr/bin/env python3
"""Run and record seeded independent I11a parser/runtime oracle campaigns."""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0002-I11a-campanas.json"
CARGO = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
SPACE_BY_MODE: dict[str, int] = {}
CAMPAIGNS = [
    ("debug", 202611091301, 100_000),
    ("release", 202611091302, 300_000),
]


def cargo_test(mode: str, *arguments: str, environment: dict[str, str] | None = None):
    command = [CARGO, "test", "--locked", "--offline"]
    if mode == "release":
        command.append("--release")
    command.extend(arguments)
    result = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        sys.stderr.write(result.stdout + result.stderr)
        raise SystemExit(result.returncode)
    free = shutil.disk_usage(ROOT).free
    SPACE_BY_MODE[mode] = min(SPACE_BY_MODE.get(mode, free), free)
    return result.stdout + result.stderr


def main() -> int:
    records = []
    for mode, seed, cases in CAMPAIGNS:
        environment = dict(os.environ)
        environment["NEXO_ORACLE_CASES"] = str(cases)
        environment["NEXO_ORACLE_SEED"] = str(seed)
        output = cargo_test(
            mode,
            "--test",
            "mock_oracle",
            "mock_oracle_differential_campaign",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
            environment=environment,
        )
        expected_batches = cases // 1_000
        evidence = f"mock_oracle_seed={seed} cases={cases} "
        if (
            evidence not in output
            or f"runtime_batches={expected_batches}" not in output
            or f"indexed_read_batches={expected_batches}" not in output
            or "work_boundary=R0001" not in output
            or "cells_profile=cells-64" not in output
            or 'file_open_categories=["missing", "directory", "unreadable"]' not in output
        ):
            sys.stderr.write(output)
            raise SystemExit(f"unexpected oracle campaign evidence: {mode}")
        coverage_match = re.search(
            r"parser_categories=(\[[0-9, ]+\]) rejection_classes=(\[[0-9, ]+\])",
            output,
        )
        if coverage_match is None:
            sys.stderr.write(output)
            raise SystemExit(f"missing parser/rejection coverage evidence: {mode}")
        parser_counts, rejection_counts = (
            json.loads(coverage_match.group(1)),
            json.loads(coverage_match.group(2)),
        )
        if len(parser_counts) != 128 or min(parser_counts) < cases // 128 or any(
            count == 0 for count in rejection_counts
        ):
            raise SystemExit(f"incomplete frontier coverage: {mode}")

        with tempfile.TemporaryDirectory(prefix=f"nexo-i11a-{mode}-", dir=ROOT) as temporary:
            schema_path = Path(temporary) / "schema.json"
            schema_environment = dict(os.environ)
            schema_environment["NEXO_SCHEMA_EVIDENCE_PATH"] = str(schema_path)
            cargo_test(
                mode,
                "--test",
                "mock_oracle",
                "mock_oracle_enumerates_schema_variants_for_each_grammar_node",
                "--",
                "--exact",
                "--nocapture",
                environment=schema_environment,
            )
            schema_evidence = json.loads(schema_path.read_text(encoding="utf-8"))
            expected_schema_categories = {
                "root_key_structure": 11,
                "version_type_and_value": 9,
                "queue_types_and_shapes": 32,
                "result_key_structure": 50,
                "result_payload_json_types": 80,
                "canonical_i64_by_slot": 120,
            }
            if (
                schema_evidence["cases"] != 302
                or schema_evidence["categories"] != expected_schema_categories
                or schema_evidence["empty_queue_acceptance_cases"] != 4
                or schema_evidence["nodes"] != 11
                or schema_evidence["i64_slots"] != 8
            ):
                raise SystemExit(f"incomplete schema enumeration: {mode} {schema_evidence}")

            document_path = Path(temporary) / "document-mutations.json"
            document_environment = dict(os.environ)
            document_environment["NEXO_DOCUMENT_MUTATION_SEED"] = str(seed)
            document_environment["NEXO_DOCUMENT_MUTATION_EVIDENCE_PATH"] = str(document_path)
            cargo_test(
                mode,
                "--test",
                "mock_oracle",
                "mock_oracle_exhaustive_document_mutation_campaign",
                "--",
                "--ignored",
                "--exact",
                "--nocapture",
                environment=document_environment,
            )
            document_evidence = json.loads(document_path.read_text(encoding="utf-8"))
            operations = document_evidence["operations"]
            if (
                document_evidence["seed"] != seed
                or document_evidence["documents"] != 7
                or document_evidence["checked"] < 500_000
                or document_evidence["boundary_cases"] != 28
                or document_evidence["boundary_categories"]
                != {
                    "array_depth_boundaries": 2,
                    "malformed_numbers": 7,
                    "object_depth_boundaries": 2,
                    "size_boundaries": 3,
                    "unicode_escapes": 9,
                    "valid_noncanonical_numbers": 5,
                }
                or any(operations[name] == 0 for name in operations)
                or not {
                    "insert_sequences",
                    "replace_two_bytes_with_sequences",
                }.issubset(operations)
                or document_evidence["accepted"] == 0
                or document_evidence["accepted_runtime_probes_per_case"] != 4
                or document_evidence["rejected"]["invalid_mock_json"] == 0
                or document_evidence["rejected"]["invalid_mock_schema"] == 0
            ):
                raise SystemExit(f"incomplete systematic mutation coverage: {mode} {document_evidence}")
            sequence_categories = document_evidence["sequence_categories"]
            required_sequences = {
                f"{category}.{operation}"
                for category in [
                    "unicode_escape_valid",
                    "unicode_escape_invalid",
                    "exponent",
                    "fraction",
                    "malformed_number",
                    "valid_number_noncanonical",
                    "container_sequence",
                    "literal_sequence",
                ]
                for operation in ["insert", "replace_two_bytes"]
            }
            if not required_sequences.issubset(sequence_categories) or any(
                sequence_categories[name] == 0 for name in required_sequences
            ):
                raise SystemExit(f"incomplete structural sequences: {mode} {sequence_categories}")

        with tempfile.TemporaryDirectory(prefix=f"nexo-i11a-structural-{mode}-", dir=ROOT) as temporary:
            structural_path = Path(temporary) / "evidence.json"
            structural_environment = dict(os.environ)
            structural_environment["NEXO_STRUCTURAL_EVIDENCE_PATH"] = str(structural_path)
            cargo_test(
                mode,
                "--test",
                "mock_oracle",
                "mock_oracle_structural_sequence_campaign_runs_in_the_normal_suite",
                "--",
                "--exact",
                environment=structural_environment,
            )
            structural_evidence = json.loads(structural_path.read_text(encoding="utf-8"))
        structural_evidence.update(
            {"active_in_normal_suite": True, "minimum_cases": 20_000}
        )
        if structural_evidence["cases"] < 20_000 or structural_evidence["boundary_cases"] != 28:
            raise SystemExit(f"active structural campaign below contract: {mode} {structural_evidence}")

        cells_output = cargo_test(
            mode,
            "--test",
            "run_cli_diagnostics_and_cells",
            "cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets",
            "--",
            "--exact",
        )
        if "test result: ok" not in cells_output:
            sys.stderr.write(cells_output)
            raise SystemExit(f"cells-64 read record matrix failed: {mode}")
        liveness_output = cargo_test(
            mode,
            "--test",
            "run_cli_diagnostics_and_cells",
            "cells64_write_and_reply_literal_arguments_keep_the_current_frontier",
            "--",
            "--exact",
        )
        if "test result: ok" not in liveness_output:
            sys.stderr.write(liveness_output)
            raise SystemExit(f"write/reply literal liveness frontier failed: {mode}")
        prior_cells_output = cargo_test(
            mode,
            "--test",
            "mock_env_json_and_read_boundaries",
            "read_err_and_mixed_record_reservations_cover_cells_64_boundaries",
            "--",
            "--exact",
        )
        if "test result: ok" not in prior_cells_output:
            sys.stderr.write(prior_cells_output)
            raise SystemExit(f"legacy cells-64 Err/mixed frontier failed: {mode}")
        cli_output = cargo_test(mode, "--test", "run_cli_diagnostics_and_cells")
        if "8 passed" not in cli_output or "0 failed" not in cli_output:
            sys.stderr.write(cli_output)
            raise SystemExit(f"val57 regression suite failed: {mode}")
        profile_output = cargo_test(mode, "--test", "runtime_profiles_and_resource_limits", "--", "--nocapture")
        if (
            "16 passed" not in profile_output
            or "1 ignored" not in profile_output
            or "0 failed" not in profile_output
            or "runtime_profile_boundary_matrix profiles=8 resources=4 points=3 cases=96 observation64_cells_preempted=2"
            not in profile_output
            or "ir_32_profile=analysis_limit_32 work_frames_cells_observations=NX-E0003 tiny_loop_reaches=R0001"
            not in profile_output
            or "entry_tick_temp_matrix primitives=4 candidates=20" not in profile_output
        ):
            sys.stderr.write(profile_output)
            raise SystemExit(f"I11a profile/resource frontier matrix failed: {mode}")
        ir_items_output = cargo_test(
            mode,
            "--test",
            "runtime_profiles_and_resource_limits",
            "normal_ir_item_limit_is_profiled_and_reports_the_declared_budget",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        )
        if "normal_ir_items_source_witness=NX-E0003 limit=262144 observed=262145" not in ir_items_output:
            sys.stderr.write(ir_items_output)
            raise SystemExit(f"normal ir_items source boundary failed: {mode}")

        profile_wiring_output = cargo_test(mode, "--test", "analysis_profiles_and_quotas")
        if "3 passed" not in profile_wiring_output or "0 failed" not in profile_wiring_output:
            sys.stderr.write(profile_wiring_output)
            raise SystemExit(f"I11a CLI profile wiring and error-route matrix failed: {mode}")

        cli_argv_output = cargo_test(
            mode,
            "--test",
            "run_argv_matrix",
            "generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle",
            "--",
            "--exact",
            "--nocapture",
        )
        if (
            "test result: ok. 1 passed" not in cli_argv_output
            or "cases=1904" not in cli_argv_output
            or "dimensions=command-option-value-position-repetition" not in cli_argv_output
        ):
            sys.stderr.write(cli_argv_output)
            raise SystemExit(f"I11a exhaustive run CLI argv matrix failed: {mode}")

        unicode_escape_output = cargo_test(
            mode,
            "--test",
            "mock_oracle",
            "every_unicode_code_unit_escape_has_the_contract_rejection_phase_in_keys_and_values",
            "--",
            "--exact",
            "--nocapture",
        )
        if any(
            marker not in unicode_escape_output
            for marker in [
                "test result: ok. 1 passed",
                "scalar_units=63488",
                "lone_surrogate_cases=4096",
                "1048576",
                "pair_key_value_cases=2097152",
            ]
        ):
            sys.stderr.write(unicode_escape_output)
            raise SystemExit(f"I11a exhaustive Unicode escape matrix failed: {mode}")

        records.append(
            {
                "batch_cases": 1_000,
                "cases": cases,
                "mode": mode,
                "parser_categories": 128,
                "parser_category_counts": parser_counts,
                "rejection_counts": {
                    "invalid_mock_json": rejection_counts[1],
                    "invalid_mock_schema": rejection_counts[2],
                    "mock_env_too_large": rejection_counts[0],
                },
                "minimum_parser_cases_per_category": min(parser_counts),
                "file_open_categories": ["missing", "directory", "unreadable"],
                "runtime_batches": expected_batches,
                "indexed_read_batches": expected_batches,
                "work_boundary": {
                    "diagnostic": "R0001",
                    "limit": 1_000_000,
                    "observed": 1_000_001,
                },
                "seed": seed,
                "campaign": "independent_parser_and_runtime_oracle",
                "structural_sequence_campaign": structural_evidence,
                "document_mutation_campaign": document_evidence,
                "schema_enumeration": schema_evidence,
                "cells_64_error_record_frontier": {
                    "case_count": 14,
                    "record_counts": list(range(28, 35)),
                    "patterns": ["all_err", "alternating_ok_err"],
                    "exact_diagnostic": "R0002",
                    "test": "mock_env_json_and_read_boundaries::read_err_and_mixed_record_reservations_cover_cells_64_boundaries",
                },
                "cells_64_read_record_matrix": {
                    "result_record_counts": [0, 1, 2, 30],
                    "record_patterns": ["ok", "err", "mixed"],
                    "outer_read_results": ["ok", "err"],
                    "frontier_observed": [64, 65],
                    "empty_ok_reservation_cells": 2,
                    "test": "run_cli_diagnostics_and_cells::cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets",
                },
                "write_reply_literal_argument_liveness": {
                    "profile": "cells-64",
                    "array_items_at_limit": 61,
                    "array_items_over_limit": 62,
                    "observed_over_limit": 65,
                    "operations": ["MockWriteI64(25)", "MockReply(200)"],
                    "normative_status": "A2 does not specify whether evaluated literal arguments remain live while reserving the result",
                    "test": "run_cli_diagnostics_and_cells::cells64_write_and_reply_literal_arguments_keep_the_current_frontier",
                },
                "prior_err_and_mixed_cell_frontier": {
                    "test": "mock_env_json_and_read_boundaries::read_err_and_mixed_record_reservations_cover_cells_64_boundaries",
                    "status": "retained_and_reexecuted",
                },
                "runtime_profile_limit_matrix": {
                    "test": "runtime_profiles_and_resource_limits::runtime_profile_table_drives_work_frames_cells_and_observation_boundaries",
                    "profiles": [
                        "f0-0002-b-v2",
                        "f0-0002-b-test-types-32-v2",
                        "f0-0002-b-test-flow-32-v2",
                        "f0-0002-b-test-flow-global-32-v2",
                        "f0-0002-b-test-effects-32-v2",
                        "f0-0002-b-test-frames-8-v2",
                        "f0-0002-b-test-cells-64-v2",
                        "f0-0002-b-test-observations-64-v2",
                    ],
                    "resources": ["work", "call_frames", "live_value_cells", "observation_bytes"],
                    "frontiers": ["L-1", "L", "L+1"],
                    "cases": 96,
                    "minimum_per_profile_resource": 3,
                    "observation64_preempts_cells64_frontier": {
                        "reason": "A2 checks cells before observation emission; the live-cell L-1/L read result attempts its read event at observation_bytes=64 and observes 79",
                        "cells_L_plus_one": "R0002 live_value_cells observed=65537",
                    },
                    "ir32_analysis_preemption": {
                        "test": "runtime_profiles_and_resource_limits::ir_32_analysis_profile_reports_its_own_limit_before_runtime_execution",
                        "analysis_limit": 32,
                        "standard_resource_witnesses": ["work", "call_frames", "live_value_cells", "observation_bytes"],
                        "preemption": "NX-E0003 ir_items limit=32 observed=33 before runtime",
                        "small_infinite_loop": "R0001 limit=1000000 observed=1000001",
                    },
                },
                "analysis_profile_cli_wiring": {
                    "test": "analysis_profiles_and_quotas::analysis_quota_profiles_are_selected_by_their_cli_identifier",
                    "boundary_test": "analysis_profiles_and_quotas::reduced_analysis_source_frontiers_are_transcribed_from_the_profile_table",
                    "error_routes_test": "analysis_profiles_and_quotas::selected_profile_is_preserved_on_input_and_mock_environment_failures",
                    "profiles": [
                        "f0-0002-b-v2",
                        "f0-0002-b-test-types-32-v2",
                        "f0-0002-b-test-flow-32-v2",
                        "f0-0002-b-test-flow-global-32-v2",
                        "f0-0002-b-test-effects-32-v2",
                        "f0-0002-b-test-ir-32-v2",
                        "f0-0002-b-test-frames-8-v2",
                        "f0-0002-b-test-cells-64-v2",
                        "f0-0002-b-test-observations-64-v2",
                    ],
                    "error_formats": ["text", "json"],
                    "diagnostic_classes": ["NX-E0003", "NX-E0004", "NX-E0102", "R0001", "unknown_profile", "mock_env_unreadable"],
                    "normative_basis": ["A2 §11.4–§11.6", "D15 §6.1", "D15 §7.1"],
                },
                "run_cli_argv_matrix": {
                    "commands": ["run", "check", "runx", "mystery"],
                    "options": ["--mock-env", "--format", "--profile"],
                    "values": [
                        "option_absent",
                        "value_absent",
                        "empty",
                        "-",
                        "-x",
                        "--",
                        "--x",
                        "published_profile_id",
                        "profile_id_other_case",
                        "existing_path",
                        "nonexistent_path",
                    ],
                    "positions": ["before_source", "between_repeated_options", "after_source"],
                    "repetition": ["once", "same_value", "different_values"],
                    "cases": 1904,
                    "comparison": "literal exit code, stdout bytes, and stderr bytes",
                    "test": "run_argv_matrix::generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle",
                    "normative_basis": ["A2 §12.4", "D15 §7.1", "D4", "coordination decision D5"],
                },
                "unicode_escape_key_value_matrix": {
                    "bmp_scalar_code_units": 63488,
                    "lone_surrogate_cases": 4096,
                    "high_low_surrogate_pairs": 1048576,
                    "pair_key_value_cases": 2097152,
                    "expected_single_surrogate_reason": "invalid_mock_json",
                    "expected_scalar_or_pair_reason": "invalid_mock_schema",
                    "test": "mock_oracle::every_unicode_code_unit_escape_has_the_contract_rejection_phase_in_keys_and_values",
                    "basis": "A2 §12.4, Esquema del entorno, ll. 1501–1538; F0-0004 §6.1 l. 229",
                },
                "normal_ir_items_source_boundary": {
                    "profile": "f0-0002-b-v2",
                    "limit": 262_144,
                    "observed": 262_145,
                    "source_case_count": 1,
                    "test": "runtime_profiles_and_resource_limits::normal_ir_item_limit_is_profiled_and_reports_the_declared_budget",
                },
                "temporary_resource_entry_tick_matrix": {
                    "test": "runtime_profiles_and_resource_limits::entry_tick_exhaustion_closes_every_temporary_resource_primitive",
                    "primitives": [
                        "MockReadRecords(MockOpen(1))?",
                        "Inspect(MockOpen(1))",
                        "Drop(MockOpen(1))",
                        "Drop(MockBlockingOpen(1))",
                    ],
                    "candidate_sources": 20,
                    "matched_entry_faults": 8,
                    "minimum_matches_per_primitive": 1,
                    "work_limit": 1_000_000,
                    "required_event_sequence": ["open:0:1", "close:0:1"],
                    "required_observation_bytes": 47,
                    "required_reserved_observation_bytes": 0,
                    "citations": ["A2 §11.2 l. 903", "A2 §11.6 l. 1068", "A2 §12.4 l. 1594"],
                },
                "cli_option_matrix": {
                    "options": ["--format", "--profile", "--mock-env"],
                    "presence_states": [
                        "absent",
                        "once",
                        "repeated_same",
                        "repeated_different",
                        "missing_value",
                        "option_like_value",
                        "unknown_option",
                        "outside_run",
                    ],
                    "formats": ["text", "json"],
                    "result_classes": [
                        "Ok",
                        "Err_payload",
                        "R0001",
                        "R0002",
                        "SimulatorFault_R0101",
                        "invalid_mock_json",
                        "invalid_mock_schema",
                        "mock_env_unreadable",
                        "mock_env_too_large",
                        "compile_error",
                    ],
                    "test": "run_cli_diagnostics_and_cells::run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings",
                },
                "unreadable_cli_shapes_and_formats": {
                    "file_shapes": ["missing", "directory"],
                    "formats": ["json", "text"],
                    "expected_reason": "mock_env_unreadable",
                    "test": "mock_env_json_and_read_boundaries::unreadable_environment_is_exact_json_and_text_for_missing_and_directory",
                    "read_error_after_open_test": "mock_env_json_and_read_boundaries::read_failure_after_open_is_reported_as_unreadable",
                },
            }
        )
        subprocess.run([CARGO, "clean"], cwd=ROOT, check=True)
    payload = json.dumps(
        {
            "cut": "I11a",
            "generator": "tests/mock_oracle.rs",
            "oracle": "two structurally distinct JSON parsers + indexed read and four-queue runtime reference models",
            "campaigns": records,
        },
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    ) + "\n"
    OUTPUT.write_bytes(payload.encode("utf-8"))
    for record in records:
        print(
            f"campaign={record['campaign']} mode={record['mode']} seed={record['seed']} "
            f"cases={record['cases']} batch={record['batch_cases']} "
            f"runtime_batches={record['runtime_batches']} "
            f"document_mutations={record['document_mutation_campaign']['checked']} "
            f"schema_variants={record['schema_enumeration']['cases']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
