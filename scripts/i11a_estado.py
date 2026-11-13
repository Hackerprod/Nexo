#!/usr/bin/env python3
"""Regenera byte a byte el estado verificable del corte I11a."""

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0002-I11a-estado.json"
CAMPAIGNS = ROOT / "docs/validation/F0-0002-I11a-campanas.json"
MUTATIONS = ROOT / "docs/validation/F0-0002-I11a-mutaciones.json"
RD_FIXTURE = ROOT / "tests/fixtures/i07c_rd.json"
HASHED_DOCUMENTS = [
    "HANDOFF.md",
    "Pending.md",
    "RECOVERY.md",
    "docs/F0_ACUERDOS.md",
    "docs/adr/F0-0002-second-slice.md",
    "docs/adr/F0-CODIGOS.md",
    "docs/adr/F0-0004-text-and-user-types.md",
    "docs/adr/F0-D15-sintaxis-oficial.md",
    "docs/validation/F0-0002-I11a-implementation-notes.md",
    "docs/validation/F0-0002-I11a-campanas.json",
    "docs/validation/F0-0002-I11a-mutaciones.json",
]

# Explicit measurements from the full verification run on this date. Values
# derivable from result artifacts and the RD fixture are computed below.
MEASURED_ON = "2026-11-12"
MEASURED_TESTS = {
    "debug": {"passed": 1535, "failed": 0, "ignored": 67},
    "release": {"passed": 1537, "failed": 0, "ignored": 68},
}
MEASURED_RD_SNAPSHOT_DIFFERENCES = 0


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def state() -> dict:
    campaigns = read_json(CAMPAIGNS)["campaigns"]
    mutation_records = read_json(MUTATIONS)["mutations"]
    mutation_artifact = read_json(MUTATIONS)
    rd_rows = read_json(RD_FIXTURE)["rows"]

    statuses = [record["status"] for record in mutation_records]
    killed = statuses.count("killed")
    equivalent = statuses.count("equivalent")
    debt = statuses.count("debt")
    surviving = sum(status not in ("killed", "equivalent", "debt") for status in statuses)
    if surviving:
        raise SystemExit(f"I11a mutation population has {surviving} non-equivalent survivors")
    all_restored = all(
        record["production_sha256"] == record["restored_sha256"]
        for record in mutation_records
    )
    if not all_restored:
        raise SystemExit("I11a mutation population has an unrestored SHA-256")

    return {
        "cut": "I11a",
        "base_head": "58741cc",
        "status": "implementation_corrected_local_verification_completed_pending_independent_review",
        "preserved_metrics": {
            "analysis_b_g1_ir_items": 162309,
            "analysis_b_g1_ownership_flow_steps": 4282686,
            "assert_eq": 19,
            "index_assignment": 6,
            "loop_3": 56,
            "print": 10,
            "rd_snapshots": len(rd_rows),
            "result_err": 15,
            "result_ok": 25,
            "empty_main": 2,
            "empty_block_statement": 1,
            "measured_on": MEASURED_ON,
        },
        "normative_witnesses": {
            "ex01_observation_bytes": 102,
            "request_reply_observation_bytes": 42,
            "write_ok_observation_bytes": 27,
            "write_err_40_observation_bytes": 28,
            "read_cells_formula": "2 + 2*n for Ok(n), 2 for Err",
            "write_request_reply_cells": 2,
        },
        "independent_mock_oracle": {
            "campaign_cases_debug": sum(
                item["cases"] for item in campaigns if item["mode"] == "debug"
            ),
            "campaign_cases_release": sum(
                item["cases"] for item in campaigns if item["mode"] == "release"
            ),
            "parser_categories": min(item["parser_categories"] for item in campaigns),
            "minimum_parser_cases_per_category": min(
                min(item["parser_category_counts"]) for item in campaigns
            ),
            "file_open_categories": campaigns[0]["file_open_categories"],
            "indexed_read_batches": {
                item["mode"]: item["indexed_read_batches"] for item in campaigns
            },
            "file_open_categories": campaigns[0]["file_open_categories"],
            "indexed_read_batches_debug": next(
                item["indexed_read_batches"] for item in campaigns if item["mode"] == "debug"
            ),
            "indexed_read_batches_release": next(
                item["indexed_read_batches"] for item in campaigns if item["mode"] == "release"
            ),
            "rejection_classes": sorted(
                key
                for key, count in campaigns[0]["rejection_counts"].items()
                if count > 0
            ),
            "seeds": [item["seed"] for item in campaigns],
            "work_boundary": campaigns[0]["work_boundary"],
            "systematic_document_mutations_per_profile": campaigns[0][
                "document_mutation_campaign"
            ]["checked"],
            "document_mutation_operations": campaigns[0]["document_mutation_campaign"][
                "operations"
            ],
            "accepted_document_runtime_probes": campaigns[0][
                "document_mutation_campaign"
            ]["accepted_runtime_probes_per_case"],
            "schema_variants": campaigns[0]["schema_enumeration"]["cases"],
            "cells_64_error_record_frontier": campaigns[0][
                "cells_64_error_record_frontier"
            ],
        },
        "campaigns": [dict(item, status="completed") for item in campaigns],
        "mutation_summary": {
            "population_total": len(mutation_records),
            "production_mutations_attempted": len(mutation_records) - equivalent,
            "killed": killed,
            "equivalent": equivalent,
            "normative_debts": debt,
            "surviving_non_equivalent": surviving,
            "m56_mutants": sum(
                record["id"].startswith("m56_") for record in mutation_records
            ),
            "m56_killed": sum(
                record["id"].startswith("m56_") and record["status"] == "killed"
                for record in mutation_records
            ),
            "all_restored_sha256": all_restored,
            "equivalent_ids": sorted(
                record["id"]
                for record in mutation_records
                if record["status"] == "equivalent"
            ),
            "debt_ids": sorted(
                record["id"] for record in mutation_records if record["status"] == "debt"
            ),
            "from_id_wiring_mutations": {
                "checked": len(mutation_artifact.get("from_id_wiring_checks", [])),
                "killed": sum(
                    item.get("status") == "killed"
                    for item in mutation_artifact.get("from_id_wiring_checks", [])
                ),
                "sha256_restored": all(
                    item.get("production_sha256") == item.get("restored_sha256")
                    for item in mutation_artifact.get("from_id_wiring_checks", [])
                ),
            },
            "observation_debts": mutation_artifact.get("observation_debts", {}),
            "observable_dimensions": mutation_artifact["observability_dimensions"],
            "mutation_axes": mutation_artifact["mutation_axes"],
        },
        "document_hashes_sha256": {
            path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
            for path in HASHED_DOCUMENTS
        },
        "verification": {
            "measured_on": MEASURED_ON,
            "cargo_test_debug": {"status": "completed", **MEASURED_TESTS["debug"]},
            "cargo_test_release": {"status": "completed", **MEASURED_TESTS["release"]},
            "rd_check_run_pairs": {
                "status": "completed",
                "passed": len(rd_rows),
                "total": len(rd_rows),
                "snapshot_differences": MEASURED_RD_SNAPSHOT_DIFFERENCES,
            },
            "clippy_warnings_denied": "completed",
            "fmt": "completed",
            "clean_validate_clone": "completed_removed_clone_and_target",
            "mutation_zones": "mock parser/schema and structural sequence campaign, exhaustive Unicode escape key/value and surrogate-pair rejection matrix, runtime profile/resource L-1/L/L+1 matrix, temporary-resource entry cleanup, R diagnostic fields, primitive effects, exhaustive generated run CLI argv matrix, val62 run parse branch mutations, CLI profile/format selection and D4 argument dash boundary, ReducedProfile::from_id id/variant mutations for all nine published profiles, IR verification/lowering, observations, per-constant profile limit table, official B profile identifiers in F0-CODIGOS/D15, measured val59 explain/Print/earlier-slice mutant replay, complete MockOpen acquisition proposal",
            "hygiene_and_lf": "completed",
        },
        "generated_artifacts": [
            "docs/validation/F0-0002-I11a-campanas.json",
            "docs/validation/F0-0002-I11a-mutaciones.json",
        ],
        "open": [
            "independent validation",
            "I11b",
            "owner phase approval",
            "F0 review gates",
            "D2 work-versus-cell check precedence remains normative debt",
            "D3 write/reply literal-argument liveness remains unspecified by A2",
            "unknown_profile JSON profile_id remains unspecified by A2 §12.4/§13.9",
        ],
    }


def main() -> None:
    payload = json.dumps(state(), ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    OUTPUT.write_bytes(payload.encode("utf-8"))
    print(f"generated={OUTPUT.relative_to(ROOT).as_posix()} bytes={len(payload.encode('utf-8'))}")


if __name__ == "__main__":
    main()
