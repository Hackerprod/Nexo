#!/usr/bin/env python3
"""One release mutation pass for the J06–J09 language core."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile

from j02_j04_mutations import ROOT, copy_tree, sha

OUTPUT = ROOT / "docs/validation/F0-0003-J06-J09-mutations.json"
MUTATIONS = [
    ("explicit_capture_boundary", "src/check/names.rs",
     "            if exhaustive {", "            if !exhaustive {",
     ["--test", "j06_names", "exhaustive_capture_list_rejects_one_free_binding"]),
    ("retained_loan_acquisition", "src/ir/lower.rs",
     "                        self.active_loans\n                            .entry(id)\n                            .or_default()\n                            .push(active.clone());",
     "                        self.active_loans\n                            .entry(id)\n                            .or_default()\n                            .extend([]);",
     ["--test", "j07_retention", "retained_view_blocks_write_until_carrier_drops"]),
    ("receiver_mode_order", "src/ir/lower.rs",
     "if mode_rank(receiver_mode) < mode_rank(required) {",
     "if mode_rank(receiver_mode) > mode_rank(required) {",
     ["--test", "j08_receiver", "read_mode_cannot_call_mutating_or_consuming_closure"]),
    ("owned_capture_drop_effect", "src/ir/effects.rs",
     'resource_effect(&ty) | closure_operation_effect(&ty, inferred, "drop", 0)',
     'closure_operation_effect(&ty, inferred, "drop", 0)',
     ["--test", "j09_effects", "written_closure_operation_bounds_use_capture_witnesses"]),
    ("nominal_type_diagnostic", "src/check/types.rs",
     'output.push_str("Closure@");', 'output.push_str("Closure");',
     ["--test", "j09_closure_fuzz", "distinct_closure_sites_use_canonical_types_in_diagnostic"]),
]


def run(command: list[str], copy: Path, target: Path) -> subprocess.CompletedProcess[bytes]:
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    return subprocess.run(command, cwd=copy, env=env, capture_output=True,
                          timeout=300, check=False)


def main() -> int:
    records = []
    with tempfile.TemporaryDirectory(prefix="nexo-j06-j09-mutations-", dir="D:/") as temporary:
        copy = Path(temporary) / "source"
        copy_tree(copy)
        target = Path(temporary) / "target"
        for mutation_id, filename, old, new, test in MUTATIONS:
            path = copy / filename
            original = path.read_bytes()
            before = sha(original)
            old_bytes = old.encode("utf-8")
            if original.count(old_bytes) != 1:
                raise RuntimeError(f"mutation site is not unique: {mutation_id}")
            mutated = original.replace(old_bytes, new.encode("utf-8"), 1)
            path.write_bytes(mutated)
            try:
                command = ["cargo", "test", "--offline", "--locked", "--release", "--quiet",
                           *test, "--", "--test-threads=1"]
                done = run(command, copy, target)
                output = (done.stdout + done.stderr).decode("utf-8", "replace")
            finally:
                path.write_bytes(original)
            if sha(path.read_bytes()) != before:
                raise RuntimeError(f"mutation was not restored: {mutation_id}")
            killed = done.returncode != 0 and ("FAILED" in output or "panicked" in output)
            records.append({"id": mutation_id, "file": filename,
                            "original_sha256": before, "mutated_sha256": sha(mutated),
                            "restored": True, "status": "killed" if killed else
                            "survived" if done.returncode == 0 else "invalid",
                            "test": " ".join(test)})
            print(f"{mutation_id}: {records[-1]['status']}", flush=True)
    OUTPUT.write_text(json.dumps({"mode": "quick-release", "mutations": records},
                                 ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return int(any(record["status"] == "invalid" for record in records))


if __name__ == "__main__":
    raise SystemExit(main())
