"""Inverse mutation of the three V6 changes of S5a.1, on a disposable copy.

Usage: python -B .../s5a1-mutate.py <copy-root> <output.json>
The copy must be a tree with the candidate `src/check/types.rs`; the file is restored by SHA-256
after each mutant. A mutant is DEAD when the S5a.1 regressions (or the edited existing tests) fail
under `cargo nextest run --profile default` (first failing test recorded); otherwise it SURVIVES
and is reported with its program (none is declared here without a program).
"""

import hashlib
import json
from pathlib import Path
import subprocess
import sys


SELECTION = "test(i07c_s5_seed_consistency) | test(i07c_pending_nominal_domain)"
MUTANTS = [
    ("M1-retraction-off", "retract_unconsumed_seeds",
     "        let nodes: Vec<u64> = self.graph.type_nodes.keys().copied().collect();\n        let mut witnessed_roots",
     "        return false;\n        #[allow(unreachable_code)]\n        let nodes: Vec<u64> = self.graph.type_nodes.keys().copied().collect();\n        let mut witnessed_roots"),
    ("M1b-everything-witnessed", "retract_unconsumed_seeds",
     "            if witnessed {\n                continue;\n            }",
     "            if witnessed || true {\n                continue;\n            }"),
    ("M1c-nothing-witnessed-by-component", "retract_unconsumed_seeds",
     "            let witnessed = witnessed_roots.contains(&root)\n                ||",
     "            let witnessed = false\n                ||"),
    ("M2a-read-plan-ignores-verdict", "collect_planned_shapes",
     "                && !self.graph.rejected_assignments.contains(&site)\n", ""),
    ("M2b-read-plan-ignores-retraction", "collect_planned_shapes",
     "                && !self.graph.seed_cap.contains_key(&site)\n", ""),
    ("M2c-verdict-only-on-error", "note_assignment_verdict",
     "&& (failed || before.is_some_and(|before| self.pending_join_count() > before))",
     "&& failed"),
    ("M2d-verdict-never-on-error", "note_assignment_verdict",
     "&& (failed || before.is_some_and(|before| self.pending_join_count() > before))",
     "&& before.is_some_and(|before| self.pending_join_count() > before)"),
    ("M3-concrete-plan-after-retraction", "seed_planned_shape",
     "        if !self.planning\n            && !retracted\n", "        if !self.planning\n"),
    ("M4-cap-ignored-when-seeding", "seed_planned_shape",
     "origins.into_iter().take(cap).enumerate()", "origins.into_iter().take(usize::MAX).enumerate()"),
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    root, output = Path(sys.argv[1]), Path(sys.argv[2])
    target = root / "src/check/types.rs"
    original = target.read_bytes()
    initial = hashlib.sha256(original).hexdigest()
    results = []
    for name, region, old, new in MUTANTS:
        text = original.decode("utf-8")
        assert text.count(old) == 1, (name, text.count(old))
        target.write_bytes(text.replace(old, new).encode("utf-8"))
        mutated = sha(target)
        process = subprocess.run(
            ["cargo", "nextest", "run", "--profile", "default", "--offline", "--no-fail-fast",
             "-E", SELECTION],
            cwd=root, capture_output=True, text=True, timeout=1500)
        failing = sorted({line.split()[-1] for line in (process.stdout + process.stderr).splitlines()
                          if line.lstrip().startswith("FAIL") and "::" in line.split()[-1]})
        compiled = "error[" not in process.stderr and "could not compile" not in process.stderr
        verdict = "DEAD" if compiled and failing else ("NOT-COMPILED" if not compiled else "SURVIVES")
        results.append({"mutant": name, "region": region, "sha256_initial": initial,
                        "sha256_mutated": mutated, "verdict": verdict, "failing_tests": failing})
        target.write_bytes(original)
        assert sha(target) == initial, "restoration failed"
        print(name, verdict, len(failing), flush=True)
    output.write_text(json.dumps({"results": results}, indent=1) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
