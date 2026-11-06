#!/usr/bin/env python3
"""Genera el estado reproducible del corte I10a a partir del árbol versionable."""
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = "6cddc5a"
FILES = (
    "src/check.rs", "src/main.rs", "src/check/names.rs", "src/diagnostic.rs",
    "src/check/tests.rs", "src/frontend/parse.rs", "src/interp/machine.rs",
    "src/ir/effects.rs", "src/ir/i05_tests.rs", "src/ir/i07a_tests.rs",
    "src/ir/i07b_tests.rs", "src/ir/i10a_tests.rs", "src/ir/lower.rs",
    "src/ir/model.rs", "src/ir/mod.rs", "src/ir/ownership_flow.rs",
    "src/ir/ownership_flow/sparse.rs", "src/ir/tests.rs", "src/ir/verify.rs",
    "tests/i01_diagnostic.rs", "scripts/i07c_mutaciones.py",
    "scripts/i10a_mutaciones.py", "scripts/i10a_estado.py",
)
OUTPUT = ROOT / "docs/validation/F0-0002-I10a-state.json"
MUTATION_RESULTS = ROOT / "docs/validation/F0-0002-I10a-mutation-results.jsonl"
TYPED_RESULTS = ROOT / "docs/validation/F0-0002-I10a-typed-campaign-results.json"
VERIFICATION_RESULTS = ROOT / "docs/validation/F0-0002-I10a-verification-results.json"


def main() -> None:
    rows = []
    for name in FILES:
        data = (ROOT / name).read_bytes()
        text = data.decode("utf-8")
        rows.append({"path": name, "sha256": hashlib.sha256(data).hexdigest(),
                     "bytes": len(data), "lf": data.count(b"\n"),
                     "crlf": data.count(b"\r"), "nuls": data.count(b"\x00"),
                     "working_tree_lines": len(text.splitlines())})
    population_path = ROOT / "docs/validation/F0-0002-I10a-mutations.json"
    population = json.loads(population_path.read_text(encoding="utf-8")) if population_path.exists() else None
    mutation_rows = [json.loads(line) for line in MUTATION_RESULTS.read_text(encoding="utf-8").splitlines() if line]
    population_mutations = population["mutations"] if population else []
    unique_mutation_keys = {
        (row["file"], row["literal"], row["replacement"], row["operator"])
        for row in population_mutations
    }
    result_by_id = {row["id"]: row for row in mutation_rows}
    result_population_matches = all(
        row["id"] in result_by_id
        and result_by_id[row["id"]]["mutation_sha256"]
        == hashlib.sha256(json.dumps(row, sort_keys=True).encode()).hexdigest()
        for row in population_mutations
    ) and len(result_by_id) == len(population_mutations)
    mutation_counts: dict[str, dict[str, int]] = {}
    for row in mutation_rows:
        for profile, result in row["result"].items():
            profile_counts = mutation_counts.setdefault(profile, {})
            profile_counts[result] = profile_counts.get(result, 0) + 1
    typed = json.loads(TYPED_RESULTS.read_text(encoding="utf-8"))
    verification = json.loads(VERIFICATION_RESULTS.read_text(encoding="utf-8"))
    status = subprocess.run(["git", "status", "--short"], cwd=ROOT, check=True,
                            capture_output=True, text=True, encoding="utf-8").stdout.splitlines()
    result = {
        "cut": "I10a", "baseline_commit": BASE,
        "source_inventory": rows,
        "mutation_population_count": len(population["mutations"]) if population else None,
        "mutation_population_unique_count": len(unique_mutation_keys),
        "mutation_results_match_population": result_population_matches,
        "mutation_population_sha256": hashlib.sha256(population_path.read_bytes()).hexdigest()
            if population_path.exists() else None,
        "mutation_results": {
            "count": len(mutation_rows),
            "sha256": hashlib.sha256(MUTATION_RESULTS.read_bytes()).hexdigest(),
            "by_profile": mutation_counts,
        },
        "typed_campaigns": typed["campaigns"],
        "verification": verification,
        "verification_results_sha256": hashlib.sha256(VERIFICATION_RESULTS.read_bytes()).hexdigest(),
        "typed_campaign_results_sha256": hashlib.sha256(TYPED_RESULTS.read_bytes()).hexdigest(),
        "working_tree_status": status,
        "test_results": "No inferido por este generador; se registra en el informe de implementación.",
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
