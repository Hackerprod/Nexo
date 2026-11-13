#!/usr/bin/env python3
"""One quick Release mutation pass over the J05 language core."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile

from j02_j04_mutations import ROOT, copy_tree, sha

OUTPUT = ROOT / "docs/validation/F0-0003-J05-mutations.json"
MUTATIONS = [
    ("guarded_pattern_cannot_cover", "src/check/types.rs",
     "            } else {\n                previous.push(&arm.pattern);\n            }",
     "            } else {\n                previous.push(&arm.pattern);\n            }\n            if arm.guard.is_some() { previous.push(&arm.pattern); }",
     ["--test", "j05_match_guards", "guarded_arm_does_not_prove_coverage_and_unmatched_guard_is_not_evaluated"]),
    ("move_intent", "src/ir/lower_match.rs",
     'Mode::Consume => Some("guard_transfer")', 'Mode::Consume => Some("guard_mutation")',
     ["--test", "j05_match_guards", "q03_move_intent_is_rejected_without_consuming_root"]),
    ("edit_intent", "src/ir/lower_match.rs",
     'Mode::Inout => Some("guard_mutation")', 'Mode::Inout => Some("guard_transfer")',
     ["--test", "j05_match_guards", "q02_edit_intent_is_rejected_without_mutating_root"]),
    ("provisional_extraction", "src/ir/lower_match.rs",
     "let moved = !source.provisional && *mode == Some(Mode::Consume);",
     "let moved = source.provisional && *mode == Some(Mode::Consume);",
     ["--test", "j05_match_guards", "false_guard_releases_provisional_binding_before_owner_extraction"]),
    ("guard_false_edge", "src/ir/lower_match.rs",
     "previous_branches.push(guard_branch);", "let _ = guard_branch;",
     ["--test", "j05_match_guards", "false_guard_releases_provisional_binding_before_owner_extraction"]),
    ("independent_effect_sites", "src/ir/effects.rs",
     "for (_, _, bit, witness) in candidates {", "for (_, _, bit, witness) in candidates.into_iter().take(1) {",
     ["--test", "j05_match_guards", "independent_guard_effect_sites_each_report_one_diagnostic"]),
]


def run(command: list[str], copy: Path, target: Path, timeout: int) -> subprocess.CompletedProcess[bytes]:
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    return subprocess.run(command, cwd=copy, env=env, capture_output=True, timeout=timeout, check=False)


def main() -> int:
    records = []
    with tempfile.TemporaryDirectory(prefix="nexo-j05-mutations-", dir="D:/") as temporary:
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
                command = ["cargo", "test", "--offline", "--locked", "--release", "--quiet", *test, "--", "--test-threads=1"]
                done = run(command, copy, target, 240)
                output = (done.stdout + done.stderr).decode("utf-8", "replace")
                if done.returncode == 0:
                    full = run(["cargo", "test", "--offline", "--locked", "--release", "--quiet", "--", "--test-threads=1"], copy, target, 1200)
                    output += (full.stdout + full.stderr).decode("utf-8", "replace")
                    done = full
            finally:
                path.write_bytes(original)
            if sha(path.read_bytes()) != before:
                raise RuntimeError(f"mutation was not restored: {mutation_id}")
            killed = done.returncode != 0 and ("FAILED" in output or "panicked" in output)
            status = "killed" if killed else "survived" if done.returncode == 0 else "invalid"
            records.append({"id": mutation_id, "file": filename,
                            "original_sha256": before, "mutated_sha256": sha(mutated),
                            "restored": True, "status": status, "test": " ".join(test)})
            print(f"{mutation_id}: {status}", flush=True)
    OUTPUT.write_text(json.dumps({"mode": "quick-release", "mutations": records},
                                 ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return int(any(record["status"] == "invalid" for record in records))


if __name__ == "__main__":
    raise SystemExit(main())
