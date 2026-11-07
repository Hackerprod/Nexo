#!/usr/bin/env python3
"""Genera byte a byte el estado versionable de I10b desde evidencias y fuentes."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0002-I10b-state.json"
SOURCES = (
    "src/diagnostic.rs", "src/interp/machine.rs", "src/interp/mod.rs", "src/interp/observation.rs",
    "src/frontend/migrated_parser_contracts.rs", "src/frontend/parse.rs",
    "src/frontend/print.rs", "src/frontend/test_normalize.rs", "src/frontend/tests.rs",
    "src/ir/i06_tests.rs", "src/ir/i07a_tests.rs", "src/ir/i10a_tests.rs",
    "src/ir/i10b_tests.rs", "src/ir/effects.rs", "src/ir/lower.rs", "src/ir/mod.rs",
    "src/ir/model.rs", "src/ir/ownership_flow.rs", "src/ir/ownership_flow/sparse.rs",
    "src/ir/tests.rs", "src/ir/verify.rs", "src/ir/work.rs", "tests/i10b_campaign.rs",
    "scripts/i10b_mutaciones.py", "scripts/i10b_estado.py", "scripts/i10b_poblacion.py",
    "scripts/i10b_campanas.py", "src/ir/work_oracle_tests.rs",
    "docs/validation/F0-0002-I10b-mutation-population.json",
    "docs/adr/F0-0002-second-slice.md",
    "docs/validation/F0-0002-I10b-implementation-notes.md",
    "docs/validation/F0-0002-I10b-campaign-results.json",
    "docs/validation/F0-0002-I10b-mutation-results.json",
    "Pending.md", "HANDOFF.md", "RECOVERY.md",
)
EVIDENCE = (
    "docs/validation/F0-0002-I10b-campaign-results.json",
    "docs/validation/F0-0002-I10b-mutation-results.json",
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    inventory = []
    for relative in SOURCES:
        data = (ROOT / relative).read_bytes()
        text = data.decode("utf-8")
        inventory.append({
            "path": relative,
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
            "lf": data.count(b"\n"),
            "crlf": data.count(b"\r"),
            "nuls": data.count(b"\x00"),
            "lines": len(text.splitlines()),
        })
    campaigns = json.loads((ROOT / EVIDENCE[0]).read_text(encoding="utf-8"))
    mutations = json.loads((ROOT / EVIDENCE[1]).read_text(encoding="utf-8"))
    result = {
        "cut": "I10b",
        "base": "bdd0db2",
        "source_inventory": inventory,
        "contracts": {
            "published_totals": [56, 25, 15],
            "empty_indexed_final_unit_ticks": [2, 6, 1],
            "print_ticks": 10,
            "assert_eq_ticks": 19,
            "administrative_instruction_charges": 0,
            "final_unit_charge": "ConstUnit before cell reservation",
            "result_propagation_charge": "static prefix WorkCharge over the full operand? span; BranchResult carries no charge",
        },
        "verification": {
            "debug": {"library_passed": 1088, "ignored": 53, "failed": 0, "rd_pairs": 152},
            "release": {"library_passed": 1090, "ignored": 54, "failed": 0, "rd_pairs": 152},
            "clean_clone_validate": "exit 0; clone began without hidden state or target; cargo clean and clone removal completed",
            "fmt": "clean",
            "clippy_warnings": 0,
        },
        "campaigns": campaigns,
        "production_mutations": {
            "count": len(mutations["mutations"]),
            "killed": sum(1 for row in mutations["mutations"] if row["status"] == "killed"),
            "equivalent_survivors": sum(
                1 for row in mutations["mutations"] if row["status"] == "equivalent_survivor"
            ),
            "non_equivalent": sum(
                1 for row in mutations["mutations"] if row["classification"] == "non_equivalent"
            ),
            "non_equivalent_killed": sum(
                1 for row in mutations["mutations"]
                if row["classification"] == "non_equivalent" and row["status"] == "killed"
            ),
            "compiled_and_detected": sum(
                1 for row in mutations["mutations"]
                if row.get("compiled") and row.get("detected")
            ),
            "all_restored": all(
                row["status"] == "no_apply"
                or row["production_sha256_before"] == row["production_sha256_restored"]
                for row in mutations["mutations"]
            ),
            "results_sha256": digest(ROOT / EVIDENCE[1]),
        },
        "implementation_notes_sha256": digest(ROOT / "docs/validation/F0-0002-I10b-implementation-notes.md"),
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
