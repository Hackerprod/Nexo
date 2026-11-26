"""Audit C0 span_key sites by function and expression, never by source line."""

import json
import hashlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src/check/types.rs"
INVENTORY = ROOT / "docs/validation/i07c-rediseno/c0-span-key-usos.json"
FUNCTION = re.compile(r"^\s*(?:(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?)?fn\s+([A-Za-z_][A-Za-z_0-9]*)")
DOMAINS = {
    "Completion",
    "ConstructorDuty",
    "FactId",
    "JoinRecord",
    "OccurrenceId",
    "Pending",
    "ResultStatus",
    "span de salida",
}


def normalized(expression):
    return " ".join(expression.split())


def sites(path):
    function = "(módulo)"
    ordinals = {}
    result = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        found = FUNCTION.match(line)
        if found:
            function = found.group(1)
        if "span_key" in line:
            expression = normalized(line)
            key = (function, expression)
            ordinals[key] = ordinals.get(key, 0) + 1
            result.append((function, expression, ordinals[key], line_number))
    return result


def audit(root=ROOT):
    inventory = json.loads((root / "docs/validation/i07c-rediseno/c0-span-key-usos.json").read_text(encoding="utf-8"))
    if inventory["base"] != "31d91dbebd31c35b3cd09095c2d8e65b96c80548":
        raise SystemExit(f"unexpected G3 base commit: {inventory['base']}")
    if inventory["archivo"] != "src/check/types.rs":
        raise SystemExit(f"unexpected G3 source file: {inventory['archivo']}")
    base_source = subprocess.check_output(
        ["git", "show", f"{inventory['base']}:{inventory['archivo']}"], cwd=ROOT
    )
    if hashlib.sha256(base_source).hexdigest() != inventory["sha256_blob"]:
        raise SystemExit("G3 inventory base blob SHA mismatch")
    if len(inventory["usos"]) != inventory["total"]:
        raise SystemExit("G3 classification row count mismatch")
    if {row["dominio_destino"] for row in inventory["usos"]} != DOMAINS:
        raise SystemExit("G3 classification domains differ from the frozen C0 table")
    expected = [
        (row["archivo"], row["funcion"], normalized(row["expresion"]), row["ordinal_misma_expresion_en_funcion"])
        for row in inventory["usos"]
    ]
    actual = [("src/check/types.rs", function, expression, ordinal) for function, expression, ordinal, _ in sites(root / "src/check/types.rs")]
    if actual != expected:
        for index, pair in enumerate(zip(expected, actual)):
            if pair[0] != pair[1]:
                raise SystemExit(f"unreviewed or changed span_key at site {index + 1}: expected {pair[0]!r}, got {pair[1]!r}")
        if len(actual) > len(expected):
            raise SystemExit(f"unreviewed or changed span_key at site {len(expected) + 1}: got {actual[len(expected)]!r}")
        raise SystemExit(f"span_key site count changed: expected {len(expected)}, got {len(actual)}")
    if len(actual) != inventory["total"]:
        raise SystemExit(f"inventory total mismatch: expected {inventory['total']}, got {len(actual)}")
    return len(actual)


def self_test():
    with tempfile.TemporaryDirectory(prefix="i07c-c0-g3-") as directory:
        root = Path(directory)
        (root / "scripts").mkdir()
        (root / "docs/validation/i07c-rediseno").mkdir(parents=True)
        (root / "src/check").mkdir(parents=True)
        shutil.copy2(Path(__file__), root / "scripts/i07c_c0_span_audit.py")
        shutil.copy2(INVENTORY, root / "docs/validation/i07c-rediseno/c0-span-key-usos.json")
        source = SOURCE.read_text(encoding="utf-8")
        target = root / "src/check/types.rs"
        target.write_text("\n\n" + source, encoding="utf-8")
        assert audit(root) == 240, "moving all source lines must preserve the audit"
        target.write_text(source + "\nfn planted_unreviewed_span() { let _ = span_key(&Span::default()); }\n", encoding="utf-8")
        try:
            audit(root)
        except SystemExit as error:
            assert "unreviewed or changed span_key" in str(error), error
        else:
            raise AssertionError("a planted raw span_key use passed the audit")


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        self_test()
        print("G3 self-test passed: line shifts accepted; planted span_key rejected")
    else:
        print(f"G3 span_key audit passed: {audit()} sites")
