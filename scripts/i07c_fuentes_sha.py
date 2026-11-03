#!/usr/bin/env python3
"""Registra las huellas de fuentes protegidas y su copia restaurada."""

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True)
    args = parser.parse_args()
    copy = args.copy.resolve()
    assert copy.is_dir() and copy != ROOT, "la copia debe ser un directorio desechable distinto del repositorio"
    population = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))
    files = sorted({item["file"] for item in population["mutations"]})
    before = {name: digest(ROOT / name) for name in files}
    copy_after = {name: digest(copy / name) for name in files}
    after = {name: digest(ROOT / name) for name in files}
    assert before == after == copy_after, "fuente o copia con sustitución activa"
    output = {
        "base": population["base"],
        "algorithm": "SHA-256 over LF-normalized bytes",
        "files": [
            {"file": name, "before": before[name], "after": after[name],
             "copy_after": copy_after[name]}
            for name in files
        ],
    }
    (DIRECTORY / "source-sha256.json").write_text(
        json.dumps(output, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8", newline="\n"
    )
    print(f"Huellas verificadas: {len(files)} fuentes")


if __name__ == "__main__":
    main()
