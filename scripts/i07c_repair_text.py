#!/usr/bin/env python3
"""Repara la doble codificación de argumentos y verifica texto versionable."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARGUMENTS = ROOT / "docs/validation/i07c-mutations/equivalence.json"
MOJIBAKE = re.compile(r"\u00c3\u0192|\u00c3\u201a|\u00ef\u00bf\u00bd|\u00c3[\u0080-\u00bf\u0192\u0160\u0152\u017d\u2018\u2019\u201c\u201d\u2022\u2013\u2014\u02dc\u2122\u0161\u203a\u0153\u017e\u0178]")
PAIR = re.compile(r"[\u00c3\u00c2][\u0080-\u00bf\u0192\u0160\u0152\u017d\u2018\u2019\u201c\u201d\u2022\u2013\u2014\u02dc\u2122\u0161\u203a\u0153\u017e\u0178]")


def repair(value: str) -> str:
    for _ in range(3):
        changed = PAIR.sub(lambda match: match.group().encode("cp1252").decode("utf-8"), value)
        if changed == value:
            return value
        value = changed
    return value


def verify() -> int:
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
    ).split(b"\0")
    checked = 0
    for raw in paths:
        if not raw:
            continue
        path = ROOT / raw.decode("utf-8")
        if path.suffix.lower() not in {".json", ".md"} or not path.is_file():
            continue
        content = path.read_text(encoding="utf-8")
        assert not MOJIBAKE.search(content), f"mojibake: {path.relative_to(ROOT)}"
        checked += 1
    return checked


def main() -> None:
    data = json.loads(ARGUMENTS.read_text(encoding="utf-8"))
    repaired = {key: repair(value) for key, value in data.items()}
    ARGUMENTS.write_text(
        json.dumps(repaired, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    assert not MOJIBAKE.search(ARGUMENTS.read_text(encoding="utf-8"))
    print(f"Texto comprobado: {verify()} JSON/MD versionables")


if __name__ == "__main__":
    main()
