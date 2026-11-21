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
        ("generate.py", 'family = index % 10\n    if family == 0:',
         'family = index % 9\n    if family == 0:', "check_generator"),
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

}


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
