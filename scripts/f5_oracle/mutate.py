"""Isolated sensitivity checks for the Python oracle itself.

Each mutant lives in a private C: copy. Original file hashes are checked both
before and after; no production Rust file or active campaign is modified.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "scripts/f5_oracle"
TMP = Path("C:/nexo-work/tmp")

# file, unique old text, replacement, targeted check
MUTANTS = {
    "F5-O1": [
        ("model.py", "return -magnitude if (a < 0) != (b < 0) else magnitude",
         "return magnitude", "check_integer_rules"),
        ("model.py", "return a - q * b", "return a + q * b", "check_integer_rules"),
        ("model.py", "return value.copy()", "return value", "check_control_and_arrays"),
        ("model.py", 'while current < hi or (stmt.op == "..=" and current == hi):',
         "while current <= hi:", "check_control_and_arrays"),
        ("charges.py", 'take_right = left if op == "and" else not left',
         'take_right = not left if op == "and" else not left', "check_charge_literals"),
        ("charges.py", "new_capacity = max(1, 2 * capacity)",
         "new_capacity = max(1, capacity + 1)", "check_charge_literals"),
        ("diagnostics.py", 'data={"pending_scope": capability}',
         'data={"pending_scope": "L00"}', "check_diagnostic_literals"),
        ("diagnostics.py", 'offset = len(source.encode("utf-8"))',
         'offset = len(source.encode("utf-8")) - 1', "check_syntax_pairs"),
    ],
    "F5-O2": [
        ("generate.py", '"+": 5, "-": 5,', '"+": 6, "-": 5,', "check_generator"),
        ("generate.py", 'rng.randint(-50, 50)', 'rng.randint(-49, 49)', "check_generator"),
        ("generate.py", 'family = index % 14\n    if family >= 10:',
         'family = index % 13\n    if family >= 10:', "check_generator"),
        ("generate.py", 'for _ in range(rng.randrange(1, 5))',
         'for _ in range(rng.randrange(1, 4))', "check_generator"),
        ("generate.py", 'hi = rng.randrange(5)', 'hi = rng.randrange(6)',
         "check_generator"),
        ("generate.py", 'value = execute(program).env["answer"]',
         'value = 0', "check_generator"),
        ("generate.py", 'code, name = "R0502", "div.zero"',
         'code, name = "R0504", "div.zero"', "check_generator"),
        ("generate.py", 'return best, attempts', 'return case, attempts',
         "check_generator"),
    ],
    "F5-O3": [
        ("runner.py", '"G1b": 3, "G2b": 5', '"G1b": 4, "G2b": 5',
         "check_runner"),
        ("runner.py", '"*", "/", "%", "<=", ">", ">=", "!="',
         '"/", "%", "<=", ">", ">=", "!="', "check_runner"),
        ("runner.py", 'add("L03b", token.start)', 'add("L04b", token.start)',
         "check_runner"),
        ("runner.py", '"lower" if missing == "L10" else "types"',
         '"types" if missing == "L10" else "types"', "check_runner"),
        ("runner.py", 'if required and "L08" not in complete:',
         'if False and "L08" not in complete:', "check_runner"),
        ("runner.py", 'return None if expected == actual else path',
         'return None if expected != actual else path', "check_runner"),
        ("runner.py", 'key == "file"', 'key == "source"', "check_runner"),
        ("runner.py", 'return range(start, end + 1)',
         'return range(start, end)', "check_runner"),
    ],
    "F5-O4": [
        ("batch.py", 'seeds = list(seed_range(spec))',
         'seeds = list(reversed(list(seed_range(spec))))', "check_batch"),
        ("batch.py", 'range(0, len(seeds), width)',
         'range(1, len(seeds), width)', "check_batch"),
        ("batch.py", 'seeds[offset:offset + width]',
         'seeds[offset:offset + width - 1]', "check_batch"),
        ("batch.py", 'len(active) != len(CAPABILITIES)',
         'len(active) < 0', "check_batch"),
        ("batch.py", 'gap_seconds < 60', 'gap_seconds < 50', "check_batch"),
        ("batch.py", '!= "C:"', '!= "D:"', "check_batch"),
        ("batch.py", 'return active', 'return ()', "check_batch"),
        ("batch.py", 'return [seeds[offset:offset + width] for offset in range(0, len(seeds), width)]',
         'return [seeds]', "check_batch"),
    ],
    "F5-O5": [
        ("model.py", "array.items.append(copy_value(value))",
         "array.items.append(value)", "check_model_pairs"),
        ("model.py", "for item in array.items[:]:",
         "for item in reversed(array.items[:]):", "check_model_pairs"),
        ("model.py", "except _Break:\n                        break\n                    finally:\n                        self.cleanup_scope()\n                    if iterable.value",
         "except _Break:\n                        pass\n                    finally:\n                        self.cleanup_scope()\n                    if iterable.value", "check_model_pairs"),
        ("model.py", 'return ("Some", array.items.pop()) if array.items else ("None",)',
         'return ("Some", array.items[0]) if array.items else ("None",)', "check_model_pairs"),
        ("model.py", "self.stdout.append(line)",
         'self.stdout.append("0")', "check_model_pairs"),
        ("model.py", "return self.call(name, args)",
         "return None", "check_model_pairs"),
        ("model.py", "self.scopes[0][name] = copy_value(value)",
         "self.scopes[0][name] = 0", "check_model_pairs"),
        ("runner.py", 'return {"stdout": vm.stdout, "events": vm.events,',
         'return {"stdout": ["0"] * len(vm.stdout), "events": vm.events,',
         "check_differential_sensitivity"),
    ],
    "F5-O6": [
        ("model.py", "target = self.reference(node.children[0])\n            old = target.read() if node.value != \"=\" else None\n            rhs = self.eval(node.children[1])",
         "rhs = self.eval(node.children[1])\n            target = self.reference(node.children[0])\n            old = target.read() if node.value != \"=\" else None", "check_model_pairs"),
        ("model.py", "lo = self.eval(iterable.children[0])\n                hi = self.eval(iterable.children[1])",
         "hi = self.eval(iterable.children[1])\n                lo = self.eval(iterable.children[0])", "check_model_pairs"),
        ("model.py", "return len(a.items) == len(b.items) and all(",
         "return all(", "check_model_pairs"),
        ("runner.py", "or len(expected) != len(actual):",
         "or len(expected) > len(actual):", "check_harness_contract"),
        ("cli.py", 'return 1 if result["differences"] else 0',
         'return 0', "check_harness_contract"),
        ("runner.py", 'if word in ("*", "/", "%", "<=", ">", ">=", "!="):',
         'if word in ("*", "/", "%", "<", ">", ">=", "!="):', "check_runner"),
        ("runner.py", '"L05b" if range_source else "L06b"',
         '"L06b" if range_source else "L05b"', "check_runner"),
        ("runner.py", 'result["diagnostics_cardinality"] = "unspecified"',
         'result["diagnostics_cardinality"] = "specified"', "check_runner"),
    ],
}

# D's exact patch files were not delivered with its report. These 24 isolated
# reconstructions cover the reported fault families; the four last cases
# intentionally demonstrate observational equivalence at this CLI surface.
MUTANTS["F5-D-described"] = (
    MUTANTS["F5-O1"][:5] + MUTANTS["F5-O2"][:5] +
    MUTANTS["F5-O3"][:5] + MUTANTS["F5-O6"][:5] + [
        ("model.py", "items = [copy_value(x) for x in args[0].items + args[1].items]",
         "items = args[0].items + args[1].items", "check_model_pairs"),
        ("model.py", "array.capacity = max(1, 2 * array.capacity)",
         "array.capacity = array.capacity + 1", "check_model_pairs"),
        ("model.py", 'if iterable.value == "..=" and current == hi:\n                        break',
         'if False and current == hi:\n                        break', "check_model_pairs"),
        ("model.py", 'if kind == "int" and not MIN_I64 <= node.value <= MAX_I64:\n                raise ModelFault("NX-E0107", "value.literal")',
         'if False:\n                raise ModelFault("NX-E0107", "value.literal")',
         "check_negative_literals"),
    ]
)


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(piece: str) -> dict[str, object]:
    cases = MUTANTS[piece]
    originals = {name: sha(PACKAGE / name) for name, *_ in cases}
    TMP.mkdir(parents=True, exist_ok=True)
    results = []
    for index, (name, old, new, check) in enumerate(cases, 1):
        with tempfile.TemporaryDirectory(prefix="f5-oracle-mut-", dir=TMP) as temporary:
            tree = Path(temporary)
            shutil.copytree(PACKAGE, tree / "scripts/f5_oracle")
            fixture_dir = tree / "tests/fixtures"
            fixture_dir.mkdir(parents=True)
            shutil.copy2(ROOT / "tests/fixtures/f5_oracle_literals.json", fixture_dir)
            shutil.copy2(ROOT / "tests/fixtures/f5_oracle_model_pairs.json", fixture_dir)
            target = tree / "scripts/f5_oracle" / name
            base_sha = sha(target)
            source = target.read_text(encoding="utf-8")
            if source.count(old) != 1:
                raise AssertionError((name, old, source.count(old)))
            target.write_text(source.replace(old, new, 1), encoding="utf-8")
            command = [sys.executable, "-B", "-c",
                       f"from scripts.f5_oracle.selftest import {check}; {check}()"]
            completed = subprocess.run(command, cwd=tree, capture_output=True,
                                       text=True, timeout=30)
            results.append({"id": f"{piece}-M{index:02d}",
                            "status": "dead" if completed.returncode else "survived",
                            "check": check, "base_sha256": base_sha,
                            "mutant_sha256": sha(target),
                            "stderr_tail": completed.stderr[-400:]})
        assert sha(PACKAGE / name) == originals[name], name
    return {"piece": piece, "dead": sum(x["status"] == "dead" for x in results),
            "survived": sum(x["status"] == "survived" for x in results),
            "mutants": results, "restored_sha256": originals}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("piece", choices=sorted(MUTANTS))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = run(args.piece)
    encoded = json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2)
    if args.output:
        args.output.write_text(encoded + "\n", encoding="utf-8")
    print(json.dumps({key: result[key] for key in ("piece", "dead", "survived")},
                     sort_keys=True))


if __name__ == "__main__":
    main()
