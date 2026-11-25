#!/usr/bin/env python3
"""Check the structural integrity of the R6 v3 frozen specification."""

import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def data(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


for name in ("r9-witness-discriminator.json", "r9-f2-variants.json", "r8-m1.json", "r8-match-main.json", "r8-f2-ajeno4.json", "r8-directed74.json", "r8-matrix45.json", "r8-t5-array5.json"):
    rows = data(name)["rows"]
    for row in rows:
        assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"], (name, row.get("id"))
    print(name, len(rows))

oracle_path = HERE.parents[2] / "scripts/i07c_rd_oracle.py"
spec = importlib.util.spec_from_file_location("i07c_rd_oracle", oracle_path)
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
for row in data("r8-matrix45.json")["rows"]:
    assert oracle.derive(row["source"])["diagnostics"] == row["expected"], row["id"]
print("matrix oracle", 45)

gates = data("r9-commit-gates.json")
for family, size in (("directed74_normative_ids", 74), ("matrix45_normative_ids", 45), ("frozen13_normative_ids", 13)):
    previous = set()
    for number in range(7):
        current = set(gates["commits"][f"C{number}"][family])
        assert previous <= current
        previous = current
    assert len(previous) == size
assert gates["directed74_unique_sources"] == 48

f2 = next(x for x in data("r8-f2-ajeno4.json")["rows"] if x["id"] == "c1-13262")
assert f2["expected_main_compact"][0][-1] == [142, 143]
assert f2["expected_code_subcause_primary_types_relation"][0][-1] == [90, 93]

migration = data("r9-test-migration.json")["rows"]
assert len(migration) == len({x["source_test"] for x in migration}) == 124
assert sum(x["class"] == "former_65" for x in migration) == 65
assert sum(x["class"] == "former_65" and x["private_checker_api"] for x in migration) == 21
assert sum(x["action"] == "conservar_sin_cambio" for x in migration) == 44

print("r9 contract structure OK")
