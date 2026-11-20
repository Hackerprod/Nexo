#!/usr/bin/env python3
"""One final J16 core mutation pass, restricted to an isolated D: checkout.

Each source is restored byte for byte before the next mutation. --replay only
rewrites the recorded evidence, so determinism can be checked without running
a second mutation campaign.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from j16_oracle import PROFILE, case, evaluate

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/F0-0003-J16-mutations.json"
MATRIX = ROOT / "docs/validation/F0-0003-J16-matrix.json"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def literal(id: str, source: str, action: str, expected: dict, section: str) -> dict:
    return case(id, source, action, expected, section)


def fixtures() -> dict[str, dict]:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    rows = {row["id"]: row for row in matrix["cases"]}
    block = "fn Main(){" + "{" * 128 + "let x=1;" + "}" * 128 + "}"
    pattern = "fn Main(){match 1{" + "(" * 128 + "_" + ")" * 128 + "=>{},}}"
    rows["mutation-block-129"] = literal("mutation-block-129", block, "check", {
        "exit": 1, "diagnostics": [{"code": "NX-E0003", "subcause": None,
                                   "primary": [137, 138], "data": {"budget_kind": "syntax_nesting",
                                   "limit": 128, "observed": 129, "budget_scope": "compilation",
                                   "incomplete_analysis": "parse"}}]}, "F0-0003 §10.3")
    rows["mutation-pattern-129"] = literal("mutation-pattern-129", pattern, "check", {
        "exit": 1, "diagnostics": [{"code": "NX-E0003", "subcause": None,
                                   "primary": [145, 146], "data": {"budget_kind": "syntax_nesting",
                                   "limit": 128, "observed": 129, "budget_scope": "compilation",
                                   "incomplete_analysis": "parse"}}]}, "F0-0003 §10.3")
    rows["mutation-pattern-binding"] = literal("mutation-pattern-binding",
        "fn Main(){let x=Some(1);match x{Some(a)=>{let z=a;},_=>{}}}", "check",
        {"exit": 0, "diagnostics": []}, "F0-0003 §§3.3,9.2")
    rows["mutation-nested-closure"] = literal("mutation-nested-closure",
        "fn Main(){let a=1;let f=fn [copy a](){let g=fn [copy a](){AssertEq(a,1);};g();};f();}",
        "run", {"exit": 0, "diagnostics": [], "outcome": "unit", "events": [], "stdout": []},
        "F0-0003 §§4.1,9.2")
    rows["mutation-grouped-array"] = literal("mutation-grouped-array",
        "fn Main(){let a=[1];let b=([1]);AssertEq(a,b);}", "check",
        {"exit": 0, "diagnostics": []}, "F0-0003 §§3.3,9.2")
    rows["mutation-next-item"] = literal("mutation-next-item",
        "gen fn G()->I64{yield 1;}fn Main(){var g=G();let a=Iterator.Next(edit g);AssertEq(a,Some(1));}",
        "run", {"exit": 0, "diagnostics": [], "outcome": "unit", "events": [], "stdout": []},
        "F0-0003 §§5,9.2")
    return rows


# A site is selected by its exact context and zero-based occurrence. The first
# six rows are the pending inversions in J16-r1; later rows exercise §13.1.
MUTATIONS = [
    ("P-block-depth", "src/frontend/parse.rs", "if self.depth > 128 {", "if self.depth > 129 {", 0,
     "mutation-block-129", "F0-0003 §10.3", None),
    ("P-type-depth", "src/frontend/parse.rs", "if self.depth > 128 {", "if self.depth > 129 {", 1,
     "type-depth-129", "F0-0003 §10.3", None),
    ("P-pattern-depth", "src/frontend/parse.rs", "if self.depth > 128 {", "if self.depth > 129 {", 2,
     "mutation-pattern-129", "F0-0003 §10.3", None),
    ("N-pattern-binding", "src/check/names.rs",
     "for arg in args.iter().rev() {\n                                    tasks.push(Walk::Pattern(arg));\n                                }",
     "for _arg in args.iter().rev() {}", 0,
     "mutation-pattern-binding", "F0-0003 §§3.3,9.2", None),
    ("T1-pattern-arity", "src/check/types.rs", "if args.len() != arity {", "if args.len() == arity {", 0,
     "F3-M04-negative", "F0-0003 §§3.3,9.1", None),
    ("T0-group-shape", "src/check/types.rs",
     "Expr::Group(inner, _) => frames.push(Frame::Enter(inner)),",
     "Expr::Group(_, _) => result = ShapeHint::Unknown,", 1,
     "mutation-grouped-array", "F0-0003 §§3.3,9.2", None),
    ("L-nested-closure", "src/ir/lower.rs", "visits.push(Visit::Block(&closure.body));",
     "let _ = &closure.body;", 0, "mutation-nested-closure", "F0-0003 §§4.1,9.2", None),
    ("F-pattern-close", "src/frontend/print.rs",
     "frames.push(Frame::Close(span.end as usize - 1));",
     "if false { frames.push(Frame::Close(span.end as usize - 1)); }", 0,
     None, "F0-D15 §3; F0-0003 §13.1",
     ["cargo", "test", "--locked", "--offline", "--test", "canonical_print",
      "accepted_corpus_files_format_like_the_canonical_printer"]),
    ("Item-present-tag", "src/interp/machine.rs", "VariantTag(u8::from(output.is_some()))",
     "VariantTag(0)", 0, "mutation-next-item", "F0-0003 §§5,9.2", None),
    ("CallOnce-mode", "src/ir/lower.rs",
     "if mode_rank(receiver_mode) < mode_rank(required) {", "if false {", 0,
     "callonce-mode-single-check", "F0-0003 §§4.2,9.2", None),
    ("active-frame-eight", "src/interp/machine.rs",
     "if self.frames >= self.limits.frames {", "if self.frames > self.limits.frames {", 1,
     "active-frames-9", "F0-0003 §7.2", None),
    ("cold-generator-work", "src/interp/machine.rs",
     "if !self.charge_work(1, &ins.span) || !self.charge_work(1, &ins.span) {",
     "if !self.charge_work(1, &ins.span) || !self.charge_work(0, &ins.span) {", 0,
     None, "F0-0003 §7.2",
     ["cargo", "test", "--locked", "--offline", "--test", "j16_integrated",
      "adr_section_seven_ticks_and_check_run"]),
    ("guard-before-extraction", "src/ir/lower_match.rs",
     "let violations = guard_violations(guard);", "let violations = Vec::<(&'static str, Span)>::new();", 0,
     "F3-Q03-negative", "F0-0003 §§3.3,9.2", None),
    ("resource-close-on-error", "src/interp/machine.rs",
     "for id in &function.cleanup {", "for id in function.cleanup.iter().take(0) {", 0,
     "A2-EX01-read-error", "F0-0003 §§4,9.2; A2 §12.1", None),
    ("observation-reservation", "src/interp/machine.rs",
     "self.log.release_reservation(open_bytes)?;", "let _ = open_bytes;", 0,
     "reservation-retained-close", "F0-0003 §§7.2,9.2", None),
    ("captured-resource-copy", "src/ir/lower.rs",
     "if !borrow && !ty.copyable() {", "if false {", 0,
     "callonce-invalid-copy-check", "F0-0003 §§4.1,9.1", None),
    ("escaping-closure-loan", "src/ir/lower.rs",
     ".find(|loan| self.locals[loan.root.0].scope == self.scope)",
     ".find(|_| false)", 0,
     "escape-wrapped-closure", "F0-0003 §§4.1,9.2", None),
]

PAIRED_FIXTURES = {
    "P-block-depth": "block-128 (same source with 127 nested blocks; accepted)",
    "P-type-depth": "type-depth-128",
    "P-pattern-depth": "pattern-128 (same source with 127 nested groups; accepted)",
    "N-pattern-binding": "F3-M01-negative",
    "T1-pattern-arity": "F3-M04-positive",
    "T0-group-shape": "F3-M03-negative",
    "L-nested-closure": "F3-C01-negative",
    "F-pattern-close": "F3-M01-positive",
    "Item-present-tag": "generator-yields-0",
    "CallOnce-mode": "callonce-consumer-positive",
    "active-frame-eight": "active-frames-8",
    "cold-generator-work": "generator-cold",
    "guard-before-extraction": "guard-before-extraction-false",
    "resource-close-on-error": "A2-EX01-sum25",
    "observation-reservation": "reservation-shared-limit",
    "captured-resource-copy": "callonce-consumer-positive",
    "escaping-closure-loan": "F3-C06-positive",
}


def replace_nth(source: str, before: str, after: str, occurrence: int) -> str:
    positions = []
    cursor = 0
    while (at := source.find(before, cursor)) >= 0:
        positions.append(at)
        cursor = at + len(before)
    if occurrence >= len(positions):
        raise RuntimeError(f"mutation anchor missing: {before[:60]} #{occurrence}")
    at = positions[occurrence]
    return source[:at] + after + source[at + len(before):]


def write_document(rows: list[dict]) -> None:
    notes = [{
        "id": "V3-resume-match-proofs",
        "status": "equivalente probable para IR emitida",
        "reason": ("verify_cfg ya valida el destino Yield→resume; la segunda arista "
                   "se usa solo para propagar pruebas de variante y no añade hechos "
                   "nuevos. No se halló programa fuente ni IR emitida válida cuyo "
                   "veredicto cambie al suprimirla. La clasificación es provisional "
                   "hasta hallar un testigo discriminante."),
    }]
    OUTPUT.write_text(json.dumps({"schema_version": 1, "mutations": rows, "notes": notes},
                                 ensure_ascii=False, sort_keys=True, indent=2) + "\n",
                      encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--replay", action="store_true")
    args = parser.parse_args()
    if args.replay:
        rows = json.loads(OUTPUT.read_text(encoding="utf-8"))["mutations"]
        write_document(rows)
        return 0
    if ROOT.drive.upper() != "D:":
        raise RuntimeError("J16 mutations run only in an isolated D: checkout")
    target = ROOT.parent / "j16b3-final-mutation-target"
    env = {**os.environ, "CARGO_TARGET_DIR": str(target)}
    fixture_by_id = fixtures()
    initial_build = subprocess.run(["cargo", "build", "--locked", "--offline", "--bin", "nexo"],
                                   cwd=ROOT, env=env, capture_output=True, timeout=180)
    if initial_build.returncode:
        raise RuntimeError("baseline mutation checkout does not build")
    rows = []
    with tempfile.TemporaryDirectory(prefix="j16-mutants-", dir=ROOT.parent) as tmp:
        directory = Path(tmp)
        for name, relative, before, after, occurrence, fixture_id, rule, test in MUTATIONS:
            path = ROOT / relative
            original = path.read_bytes()
            source = original.decode("utf-8")
            mutated = replace_nth(source, before, after, occurrence).encode("utf-8")
            fixture = fixture_by_id.get(fixture_id) if fixture_id else None
            baseline = None
            if fixture is not None:
                baseline = evaluate(target / "debug/nexo.exe", fixture, directory)
                if baseline["status"] != "PASA":
                    raise RuntimeError(f"baseline fixture differs from ADR: {fixture_id}: {baseline}")
            paired_id = PAIRED_FIXTURES[name]
            paired = fixture_by_id.get(paired_id)
            paired_status = None
            if paired is not None:
                paired_result = evaluate(target / "debug/nexo.exe", paired, directory)
                paired_status = paired_result["status"]
                if paired_status != "PASA":
                    raise RuntimeError(f"paired fixture differs from ADR: {paired_id}: {paired_result}")
            try:
                path.write_bytes(mutated)
                build = subprocess.run(["cargo", "build", "--locked", "--offline", "--bin", "nexo"],
                                       cwd=ROOT, env=env, capture_output=True, timeout=180)
                if build.returncode:
                    status = "inválido"
                    observed = {"build_exit": build.returncode}
                elif test is not None:
                    result = subprocess.run(test, cwd=ROOT, env=env, capture_output=True,
                                            timeout=180)
                    failed_build = b"could not compile" in result.stderr or b"error[E" in result.stderr
                    status = ("inválido" if failed_build else
                              "muerto" if result.returncode else "superviviente")
                    observed = {"test_exit": result.returncode}
                else:
                    result = evaluate(target / "debug/nexo.exe", fixture, directory)
                    status = "muerto" if result["status"] == "FALLA" else "superviviente"
                    data = result["observed"]
                    observed = {key: data[key] for key in
                                ("exit", "outcome", "events", "stdout", "diagnostics") if key in data}
            finally:
                path.write_bytes(original)
            restored = path.read_bytes()
            if digest(restored) != digest(original):
                raise RuntimeError(f"SHA-256 restoration failed: {name}")
            restored_build = subprocess.run(
                ["cargo", "build", "--locked", "--offline", "--bin", "nexo"],
                cwd=ROOT, env=env, capture_output=True, timeout=180)
            if restored_build.returncode:
                raise RuntimeError(f"restored checkout does not build: {name}")
            row = {"id": name, "site": relative, "anchor": before,
                   "adr_rule": rule, "fixture": fixture_id or test,
                   "paired_fixture": PAIRED_FIXTURES[name],
                   "paired_expected": paired["expected"] if paired else None,
                   "paired_baseline": paired_status,
                   "expected": fixture["expected"] if fixture else "test exits 0",
                   "baseline": baseline["status"] if baseline else "test exits 0",
                   "mutated_observed": observed, "status": status,
                   "sha_initial": digest(original), "sha_mutated": digest(mutated),
                   "sha_restored": digest(restored)}
            rows.append(row)
            print(f"{name}: {status}", flush=True)
    write_document(rows)
    summary = {status: sum(row["status"] == status for row in rows)
               for status in ("muerto", "superviviente", "equivalente", "inválido")}
    print(json.dumps(summary, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
