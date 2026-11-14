#!/usr/bin/env python3
"""Single quick mutation pass over J10–J13 language rules."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile

from j02_j04_mutations import ROOT, copy_tree, sha

OUTPUT = ROOT / "docs/validation/F0-0003-J10-J13-mutations.json"
MUTATIONS = [
    ("generator_layout_cycle", "src/check/types.rs",
     "if index == root {", "if index != root {",
     ["--test", "j10_generator", "generator_layout_rejects_direct_recursive_state"]),
    ("yield_continuation", "src/ir/lower.rs",
     "value: item,\n                    resume,\n                    span: span.clone(),",
     "value: item,\n                    resume: BlockId(0),\n                    span: span.clone(),",
     ["--test", "j11_generator", "next_returns_one_item_then_repeated_none"]),
    ("captured_generator_drop_effect", "src/ir/effects.rs",
     '"drop" => generator_drop_bits(&field, functions, inferred, &mut HashSet::new()),',
     '"drop" => 0,',
     ["--test", "j12_generator", "generator_drop_effect_transits_through_an_owning_closure"]),
    ("for_option_branch", "src/ir/lower.rs",
     "local: option,\n                    path: Vec::new(),\n                    tag: crate::union::VariantTag(1),",
     "local: option,\n                    path: Vec::new(),\n                    tag: crate::union::VariantTag(0),",
     ["--test", "j13_iteration", "for_moves_items_and_completes_generator"]),
]


def main() -> int:
    records = []
    with tempfile.TemporaryDirectory(prefix="nexo-j10-j13-mutations-", dir="D:/") as temporary:
        copy = Path(temporary) / "source"
        copy_tree(copy)
        target = Path(temporary) / "target"
        for mutation_id, filename, old, new, test in MUTATIONS:
            path = copy / filename
            original = path.read_bytes()
            old_bytes = old.encode("utf-8")
            if original.count(old_bytes) != 1:
                raise RuntimeError(f"mutation site is not unique: {mutation_id}")
            mutated = original.replace(old_bytes, new.encode("utf-8"), 1)
            path.write_bytes(mutated)
            try:
                command = ["cargo", "test", "--offline", "--locked", "--quiet",
                           *test, "--", "--test-threads=1"]
                environment = dict(os.environ, CARGO_TARGET_DIR=str(target))
                done = subprocess.run(command, cwd=copy, env=environment,
                                      capture_output=True, timeout=900, check=False)
                output = (done.stdout + done.stderr).decode("utf-8", "replace")
            finally:
                path.write_bytes(original)
            if sha(path.read_bytes()) != sha(original):
                raise RuntimeError(f"mutation was not restored: {mutation_id}")
            status = ("survived" if done.returncode == 0 else
                      "killed" if "FAILED" in output or "panicked" in output else "invalid")
            records.append({"id": mutation_id, "file": filename,
                            "original_sha256": sha(original),
                            "mutated_sha256": sha(mutated), "restored": True,
                            "status": status, "test": " ".join(test)})
            print(f"{mutation_id}: {status}", flush=True)
    OUTPUT.write_text(json.dumps({"mode": "quick-debug", "mutations": records},
                                 ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return int(any(record["status"] == "invalid" for record in records))


if __name__ == "__main__":
    raise SystemExit(main())
