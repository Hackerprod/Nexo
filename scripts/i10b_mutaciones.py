#!/usr/bin/env python3
"""Apply the fixed I10b production population one mutant at a time."""
from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
POPULATION = ROOT / "docs/validation/F0-0002-I10b-mutation-population.json"
OUTPUT = ROOT / "docs/validation/F0-0002-I10b-mutation-results.json"
FAILED_TEST = re.compile(r"test ([\w:]+) \.\.\. FAILED")
FAILED_TEST_BLOCK = re.compile(r"---- ([\w:]+) stdout ----")

# Rebased witnesses for the fifteen non-equivalent production mutations.
REBASE = {
    "low_ticks_2219": {
        "file": "src/ir/work.rs",
        "before": "event: WorkEvent::InoutPrefix,\n        units: 1,",
        "after": "event: WorkEvent::InoutPrefix,\n        units: 2,",
    },
    "low_while_cond_origin": {
        "file": "src/ir/lower.rs",
        "before": "WorkEvent::WhileCondition,\n                condition.span().clone(),",
        "after": "WorkEvent::WhileCondition,\n                span.clone(),",
    },
    "low_ifwhile_origin_2838": {
        "file": "src/ir/lower.rs",
        "before": "let origin = self\n                        .tokens\n                        .iter()\n                        .find(|token| token.span.start == span.start)\n                        .map_or_else(|| span.clone(), |token| token.span.clone());",
        "after": "let origin = span.clone();",
    },
    "low_assign_origin_2855": {
        "file": "src/ir/lower.rs",
        "before": "WorkEvent::Assignment,\n                        place.span().clone(),",
        "after": "WorkEvent::Assignment,\n                        statement_span(statement),",
    },
    "low_pre_order_idx": {
        "file": "src/ir/lower.rs",
        "before": "            out.push(span.clone());\n            collect(target, out);",
        "after": "            collect(target, out);\n            out.push(span.clone());",
    },
    "mach_938": {
        "file": "src/interp/machine.rs",
        "before": "charge_work(equal_pairs(a, b), &ins.span)",
        "after": "charge_work(equal_pairs(a, b) + 1, &ins.span)",
    },
    "mach_1286": {
        "file": "src/interp/machine.rs",
        "before": "self.dispose(old)?;\n                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {",
        "after": "self.dispose(old)?;\n                        if !self.charge_work(2, &ins.span) || !self.cell(&ins.span) {",
    },
    "eq_result_tag": {
        "file": "src/interp/machine.rs",
        "before": "if a != b {\n                2\n            } else {\n                2u64.saturating_add(equal_pairs(x, y))",
        "after": "if false {\n                2\n            } else {\n                2u64.saturating_add(equal_pairs(x, y))",
    },
    "ver_admin_rule_b": {
        "file": "src/ir/verify.rs",
        "before": ") && !ins.charges.is_empty()\n            {\n                return Err(\"work charge rule\");",
        "after": ") && false\n            {\n                return Err(\"work charge rule\");",
        "occurrence": 1,
    },
    "ver_site_selspan": {
        "file": "src/ir/verify.rs",
        "before": "valid_span(function, s)\n                && s.start < s.end",
        "after": "true\n                && s.start < s.end",
    },
    "ver_site_contain_lo": {
        "file": "src/ir/verify.rs",
        "before": "site.context.start <= s.start",
        "after": "true",
    },
    "ver_site_contain_hi": {
        "file": "src/ir/verify.rs",
        "before": "s.end <= site.context.end",
        "after": "true",
    },
    "ver_site_order": {
        "file": "src/ir/verify.rs",
        "before": "w[0].start < w[1].start && w[0].end <= w[1].start",
        "after": "w[0].start < w[1].start",
    },
    "hash_site_ctx": {
        "file": "src/ir/model.rs",
        "before": "        hash_span(bytes, &site.context, base);",
        "after": "        let _ = &site.context;",
    },
    "hash_site_sel": {
        "file": "src/ir/model.rs",
        "before": "for selector in &site.selectors {\n            hash_span(bytes, selector, base);",
        "after": "for selector in site.selectors.iter().take(0) {\n            hash_span(bytes, selector, base);",
    },
    "low_final_unit_origin": {
        "file": "src/ir/lower.rs",
        "before": "WorkEvent::FinalUnit,\n                    final_unit_origin.clone(),",
        "after": "WorkEvent::FinalUnit,\n                    span.clone(),",
    },
    "low_assign_origin_root": {
        "file": "src/ir/lower.rs",
        "before": "WorkEvent::IfStatement, origin",
        "after": "WorkEvent::IfStatement, span.clone()",
    },
    "low_skip_index_levels": {
        "file": "src/ir/lower.rs",
        "before": "index_access_spans(e).into_iter().skip(1)",
        "after": "index_access_spans(e).into_iter().skip(0)",
    },
    "mach_term_charge_ticks": {
        "file": "src/interp/machine.rs",
        "before": "if !self.charge_work(charge.ticks, &charge.origin) {",
        "after": "if !self.charge_work(charge.ticks + 1, &charge.origin) {",
        "occurrence": 2,
    },
}

for mutant_id, event_name in {
    "low_ticks_517": "FinalUnit",
    "low_ticks_1314": "WhileCondition",
    "low_ticks_2278": "ReadName",
    "low_ticks_2826": "ExpressionStatement",
    "low_ticks_2830": "EmptyValue",
    "low_ticks_2842": "IfStatement",
    "low_ticks_2877": "Assignment",
    "low_ticks_2975": "Binding",
    "low_ticks_3573": "IndexExpression",
    "low_ticks_2274": "ReadSelector",
    "low_ticks_3169": "PlaceSelector",
    "low_ticks_3570": "IfExpression",
    "low_ticks_2820": "ReturnStatement",
    "low_ticks_2839": "Block",
    "low_ticks_2856": "Entry",
}.items():
    REBASE[mutant_id] = {
        "file": "src/ir/work.rs",
        "before": f"event: WorkEvent::{event_name},\n        units: 1,",
        "after": f"event: WorkEvent::{event_name},\n        units: 2,",
    }


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def ignored(_: str, names: list[str]) -> set[str]:
    return {name for name in names if name.startswith(".") or name in {"target", "__pycache__"}}


def canonicalize_existing() -> None:
    population = json.loads(POPULATION.read_text(encoding="utf-8"))["mutants"]
    previous = json.loads(OUTPUT.read_text(encoding="utf-8"))
    reason_by_id = {row["id"]: row.get("equivalence_reason") for row in population}
    rows = previous["mutations"]
    for row in rows:
        row["equivalence_reason"] = reason_by_id[row["id"]]
    result = {
        "schema": "i10b-mutation-results-v2",
        "population_count": len(population),
        "mutations": sorted(rows, key=lambda row: row["id"]),
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                      encoding="utf-8", newline="\n")


def mutate(path: Path, spec: dict) -> tuple[bytes | None, bytes | None, str | None]:
    original = path.read_bytes()
    text = original.decode("utf-8")
    before, after = spec["before"], spec["after"]
    if spec.get("kind") == "line":
        lines = text.splitlines(keepends=True)
        candidates = [index for index, line in enumerate(lines) if before in line]
        if not candidates:
            return None, None, "line mutation site not found"
        target = spec["line"] - 1
        index = min(candidates, key=lambda candidate: (abs(candidate - target), candidate))
        lines[index] = lines[index].replace(before, after, 1)
        return original, "".join(lines).encode("utf-8"), None
    occurrence = spec.get("occurrence")
    count = text.count(before)
    if occurrence is not None:
        if count < occurrence:
            return None, None, f"expected occurrence {occurrence}, found {count}"
        start = -1
        for _ in range(occurrence):
            start = text.find(before, start + 1)
        text = text[:start] + after + text[start + len(before):]
    else:
        if count != 1:
            return None, None, f"expected one mutation site, found {count}"
        text = text.replace(before, after, 1)
    return original, text.encode("utf-8"), None


def main() -> None:
    if sys.argv[1:] == ["--canonicalize-existing"]:
        canonicalize_existing()
        return
    if sys.argv[1:]:
        raise SystemExit("usage: i10b_mutaciones.py [--canonicalize-existing]")
    population = json.loads(POPULATION.read_text(encoding="utf-8"))["mutants"]
    paths = {row["file"] for row in population} | {spec["file"] for spec in REBASE.values()}
    original_hashes = {relative: sha256((ROOT / relative).read_bytes()) for relative in paths}
    results = []
    with tempfile.TemporaryDirectory(prefix="nexo-i10b-mutation-") as temporary:
        parent = Path(temporary).resolve()
        clone = parent / "tree"
        if clone.resolve().parent != parent:
            raise RuntimeError("mutation copy escaped its temporary parent")
        shutil.copytree(ROOT, clone, ignore=ignored)
        environment = __import__("os").environ.copy()
        environment["CARGO_TARGET_DIR"] = str((clone / "target").resolve())
        for item in population:
            mid = item["id"]
            spec = {**item, **REBASE.get(mid, {})}
            if mid in REBASE:
                spec["kind"] = "string"
            path = clone / spec["file"]
            original, mutated, no_apply = mutate(path, spec)
            if no_apply:
                results.append({"id": mid, "file": spec["file"], "status": "no_apply",
                                "classification": item["prior_classification"], "reason": no_apply,
                                "equivalence_reason": item.get("equivalence_reason")})
                print(f"{mid}: no_apply", flush=True)
                continue
            original_hash = sha256(original)
            mutated_hash = sha256(mutated)
            path.write_bytes(mutated)
            completed = subprocess.run(["cargo", "test", "--offline", "--release", "--quiet"],
                                       cwd=clone, env=environment, capture_output=True, text=True,
                                       encoding="utf-8", errors="replace", check=False)
            output = completed.stdout + completed.stderr
            compiled = "could not compile" not in output and not re.search(r"error\[E\d+\]", output)
            path.write_bytes(original)
            restored_hash = sha256(path.read_bytes())
            detected = completed.returncode != 0 and compiled
            classification = item["prior_classification"]
            if detected:
                status = "killed"
            elif not compiled:
                status = "compile_error"
            elif classification == "equivalent":
                status = "equivalent_survivor"
            else:
                status = "survived"
            row = {
                "id": mid,
                "file": spec["file"],
                "production_sha256_before": original_hash,
                "production_sha256_mutated": mutated_hash,
                "production_sha256_restored": restored_hash,
                "status": status,
                "classification": classification,
                "equivalence_reason": item.get("equivalence_reason"),
                "compiled": compiled,
                "detected": detected,
                "exit_code": completed.returncode,
                "failed_tests": sorted(set(FAILED_TEST.findall(output) + FAILED_TEST_BLOCK.findall(output))),
            }
            results.append(row)
            print(f"{mid}: {status}", flush=True)
            if restored_hash != original_hash:
                raise RuntimeError(f"{mid}: production source was not restored by SHA-256")
        for relative, expected in original_hashes.items():
            if sha256((ROOT / relative).read_bytes()) != expected:
                raise RuntimeError(f"repository source changed during mutation: {relative}")
        shutil.rmtree(clone)
    result = {"schema": "i10b-mutation-results-v2", "population_count": len(population),
              "mutations": sorted(results, key=lambda row: row["id"])}
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                      encoding="utf-8", newline="\n")
    if len(results) != len(population):
        raise SystemExit("mutation result count differs from population")
    required = {row["id"] for row in population if row["prior_classification"] == "non_equivalent"}
    by_id = {row["id"]: row for row in results}
    if any(by_id[mid]["status"] != "killed" for mid in required):
        raise SystemExit("a required non-equivalent production mutant survived")


if __name__ == "__main__":
    main()
