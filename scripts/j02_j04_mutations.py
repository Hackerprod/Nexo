#!/usr/bin/env python3
"""One fast release mutation pass over the J02-J04 language core."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0003-J02-J04-mutations.json"
MUTATIONS = [
    ("option_payload_copyability", "src/ir/model.rs",
     "first.copyable() && second.as_ref().is_none_or(|ty| ty.copyable())",
     "first.copyable() || second.as_ref().is_none_or(|ty| ty.copyable())",
     ["--test", "j02_option", "option_resource_payload_is_closed_once_and_none_remains_noncopyable"]),
    ("option_none_coverage", "src/check/match_coverage.rs",
     'name == "None" && matches!(ty, Type::Union(UnionKind::Option, ..))',
     'name == "None" && matches!(ty, Type::Union(UnionKind::Result, ..))',
     ["--test", "j03_match", "match_option_missing_none_is_m01"]),
    ("variant_selection_runtime", "src/interp/machine.rs",
     "if actual == tag));", "if actual != tag));",
     ["--test", "j04_match_ownership", "owner_extraction_and_cleanup_follow_selected_arm"]),
    ("pattern_edit_permission", "src/ir/lower_match.rs",
     "selection.mode != Some(Mode::Inout)", "selection.mode == Some(Mode::Inout)",
     ["--test", "j04_match_ownership", "m06_pattern_edit_requires_exclusive_selection"]),
    ("payload_access_proof", "src/ir/verify.rs",
     "if !facts.contains(&(*local, prefix.clone(), *tag)) {",
     "if facts.contains(&(*local, prefix.clone(), *tag)) {",
     ["--lib", "match_projection_requires_the_selected_variant_on_every_incoming_edge"]),
]


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def copy_tree(destination: Path) -> None:
    shutil.copytree(ROOT, destination, ignore=shutil.ignore_patterns(
        ".git", "target", ".scratch", "__pycache__", ".i11a-mutation-copy"))


def main() -> int:
    records = []
    with tempfile.TemporaryDirectory(prefix="nexo-j-mutations-") as temporary:
        copy = Path(temporary) / "source"
        copy_tree(copy)
        for mutation_id, filename, old, new, target in MUTATIONS:
            path = copy / filename
            original = path.read_bytes()
            old_bytes, new_bytes = old.encode(), new.encode()
            if original.count(old_bytes) != 1:
                raise RuntimeError(f"mutation site is not unique: {mutation_id}")
            mutated = original.replace(old_bytes, new_bytes, 1)
            path.write_bytes(mutated)
            try:
                done = subprocess.run(
                    ["cargo", "test", "--offline", "--locked", "--release", "--quiet",
                     *target, "--", "--test-threads=1"],
                    cwd=copy, capture_output=True, timeout=180, check=False)
            finally:
                path.write_bytes(original)
            restored = sha(path.read_bytes()) == sha(original)
            if not restored:
                raise RuntimeError(f"mutation was not restored: {mutation_id}")
            output = (done.stdout + done.stderr).decode("utf-8", "replace")
            killed = done.returncode != 0 and ("FAILED" in output or "panicked" in output)
            records.append({"id": mutation_id, "file": filename,
                            "original_sha256": sha(original), "mutated_sha256": sha(mutated),
                            "restored": restored, "status": "killed" if killed else
                            "survived" if done.returncode == 0 else "invalid",
                            "test": " ".join(target)})
            print(f"{mutation_id}: {records[-1]['status']}", flush=True)
    OUTPUT.write_text(json.dumps({"mode": "quick-release", "mutations": records},
                                 ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return int(any(record["status"] == "invalid" for record in records))


if __name__ == "__main__":
    raise SystemExit(main())
