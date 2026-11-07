#!/usr/bin/env python3
"""Run fixed-seed I10b campaigns and write canonical semantic summaries."""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0002-I10b-campaign-results.json"
TEST = ROOT / "src/ir/work_oracle_tests.rs"
CASES = (("debug", 100_000, False, "008311061106c0de"),
         ("release", 300_000, True, "008411061106c0de"))
CATALOG_ROW_ORDER = [
    "entry", "final_unit", "while_condition", "inout_prefix", "read_selector",
    "read_name", "place_selector", "return_statement", "value_statement_explicit",
    "expression_statement", "if_statement", "block", "assignment", "binding",
    "if_expression_implicit", "index_expression",
]
RESULT = re.compile(r"generator=i10b-owned-tree-v2 seed=(\d+) count=(\d+) coverage=\[([^]]+)\] catalog_coverage=\[([^]]+)\]")
GENERATOR_VERSION = "i10b-owned-tree-v2"


def run(profile: str, count: int, release: bool, seed: str) -> dict:
    command = ["cargo", "test", "--offline"]
    if release:
        command.append("--release")
    command += ["--lib", "generated_sources_match_owned_tree_cost_oracle", "--", "--ignored", "--nocapture"]
    environment = os.environ.copy()
    environment["NEXO_I10B_CAMPAIGN_COUNT"] = str(count)
    environment["NEXO_I10B_CAMPAIGN_SEED"] = seed
    completed = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True,
                               encoding="utf-8", errors="replace", check=False)
    output = completed.stdout + completed.stderr
    if completed.returncode:
        raise RuntimeError(f"{profile} campaign failed with exit {completed.returncode}: {output[-4000:]}")
    match = RESULT.search(output)
    if not match:
        raise RuntimeError(f"{profile} campaign did not report deterministic counters")
    seed, actual_count = int(match.group(1)), int(match.group(2))
    distribution = [int(part.strip()) for part in match.group(3).split(",")]
    catalog_distribution = [int(part.strip()) for part in match.group(4).split(",")]
    if (actual_count != count or len(distribution) != 3 or min(distribution) < 50
            or len(catalog_distribution) != 16 or min(catalog_distribution) < 50):
        raise RuntimeError(f"{profile} campaign count/row coverage mismatch")
    return {"profile": profile, "sources": actual_count, "seed_hex": f"0x{seed:016x}",
            "expression_coverage": distribution, "catalog_row_coverage": catalog_distribution,
            "catalog_row_order": CATALOG_ROW_ORDER,
            "catalog_rows": 16, "unreachable_catalog_rows": ["EmptyValue"],
            "outcome": "completed"}


def main() -> None:
    rows = [run(profile, count, release, seed) for profile, count, release, seed in CASES]
    if rows[0]["seed_hex"] == rows[1]["seed_hex"]:
        raise RuntimeError("debug and release campaign seeds must differ")
    result = {
        "cut": "I10b",
        "generator": GENERATOR_VERSION,
        "generator_sha256": hashlib.sha256(TEST.read_bytes()).hexdigest(),
        "campaigns": rows,
        "oracle": "separately generated source tree evaluated by its own work function",
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                      encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
