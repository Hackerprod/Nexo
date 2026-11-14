#!/usr/bin/env python3
"""Run one fast J14 mutation pass and restore each source byte for byte."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0003-J14-mutations.json"
MUTANTS = [
    ("generator_header_work", "src/interp/machine.rs",
     "// The source call/name were charged by lowering. A cold\n                        // construction still pays its entry and state header.\n                        if !self.charge_work(1, &ins.span) || !self.charge_work(1, &ins.span)",
     "// The source call/name were charged by lowering. A cold\n                        // construction still pays its entry and state header.\n                        if !self.charge_work(1, &ins.span) || !self.charge_work(0, &ins.span)",
     "published_new_operation_work_totals"),
    ("next_advance_work", "src/interp/machine.rs",
     "Op::ResumeGenerator { receiver, .. } => {\n                        if !self.charge_work(1, &ins.span)",
     "Op::ResumeGenerator { receiver, .. } => {\n                        if !self.charge_work(0, &ins.span)",
     "published_new_operation_work_totals"),
    ("next_tag_reservation", "src/interp/machine.rs",
     "// Keep the output tag live while the body completes and\n                        // disposes its slots; completion may not free room first.\n                        if !self.cell(&ins.span)",
     "// Keep the output tag live while the body completes and\n                        // disposes its slots; completion may not free room first.\n                        if false && !self.cell(&ins.span)",
     "next_output_tag_coexists_with_suspended_generator_headers"),
    ("cold_empty_slot_cells", "src/interp/machine.rs",
     "Self::Generator(generator) => {\n                1 + generator\n                    .slots",
     "Self::Generator(generator) => {\n                1 + generator.slots.len() as u64 + generator\n                    .slots",
     "cold_empty_slots_do_not_release_phantom_cells_on_drop"),
    ("public_emission", "src/ir/lower.rs",
     "    Ok(program)\n}\n\n#[cfg(test)]\n#[allow(dead_code)]",
     "    if ast.functions.iter().any(|f| f.is_generator) || !inferred.closures.is_empty() {\n        return Err(CompileError::InvalidIr(\"mutant emission gate\"));\n    }\n    Ok(program)\n}\n\n#[cfg(test)]\n#[allow(dead_code)]",
     "published_new_operation_work_totals"),
]


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    records = []
    for name, relative, before, after, test in MUTANTS:
        path = ROOT / relative
        original = path.read_bytes()
        current = original.decode("utf-8")
        if current.count(before) != 1:
            raise RuntimeError(f"{name}: mutation anchor is not unique")
        mutated = current.replace(before, after, 1).encode("utf-8")
        try:
            path.write_bytes(mutated)
            run = subprocess.run(
                ["cargo", "test", "--offline", "--locked", "--test", "j14_accounting", test],
                cwd=ROOT, env=os.environ.copy(), capture_output=True, text=True,
                encoding="utf-8", errors="replace", timeout=120, check=False,
            )
            output = run.stdout + run.stderr
            status = ("invalid" if "could not compile" in output or "error[E" in output
                      else "killed" if run.returncode != 0 else "survived")
        finally:
            path.write_bytes(original)
        restored = path.read_bytes()
        if restored != original:
            raise RuntimeError(f"{name}: SHA restoration failed")
        records.append({"name": name, "site": relative, "fixture": test,
                        "expected": "fixture fails", "status": status,
                        "production_sha256": digest(original),
                        "mutant_sha256": digest(mutated),
                        "restored_sha256": digest(restored)})
        print(f"{name}: {status}", flush=True)
    OUTPUT.write_text(json.dumps({"mutations": records}, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    if any(row["status"] != "killed" for row in records):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
