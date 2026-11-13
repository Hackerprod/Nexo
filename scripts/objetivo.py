#!/usr/bin/env python3
"""Mide el avance del corpus objetivo de Nexo."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "corpus" / "objetivo"
INDEX = CORPUS / "INDEX.md"
HANDOFF = ROOT / "HANDOFF.md"
TIMEOUT_SECONDS = 10


def build_binary() -> Path:
    binary = ROOT / "target" / "release" / ("nexo.exe" if sys.platform == "win32" else "nexo")
    if not binary.is_file():
        result = subprocess.run(
            ["cargo", "build", "--locked", "--offline", "--release"],
            cwd=ROOT,
            check=False,
        )
        if result.returncode != 0:
            raise RuntimeError("cargo build --release falló")
    if not binary.is_file():
        raise RuntimeError("cargo no produjo target/release/nexo")
    return binary


def diagnostic_reason(payload: object) -> str:
    if not isinstance(payload, dict):
        return "diagnóstico JSON no interpretable"
    diagnostics = payload.get("diagnostics", [])
    if diagnostics:
        first = diagnostics[0]
        if isinstance(first, dict):
            code = str(first.get("code", "diagnóstico"))
            subcause = first.get("subcause")
            feature = first.get("feature_id")
            detail = code
            if subcause:
                detail += "/" + str(subcause)
            elif feature:
                detail += "/" + str(feature)
            return detail
    outcome = payload.get("outcome")
    if isinstance(outcome, dict) and outcome.get("kind") == "driver_error":
        return str(outcome.get("reason", "driver_error"))
    return "rechazo sin diagnóstico estructurado"


def run_one(binary: Path, source: Path) -> tuple[bool, str]:
    env_path = source.with_suffix(".env.json")
    common = [str(binary), "check", str(source), "--format", "json"]
    try:
        check = subprocess.run(common, cwd=ROOT, capture_output=True, text=True,
                               encoding="utf-8", errors="replace", timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        return False, "check: tiempo de espera (10 s)"
    try:
        checked = json.loads(check.stdout)
    except json.JSONDecodeError:
        return False, "check: salida JSON inválida"
    if check.returncode != 0 or checked.get("outcome", {}).get("kind") != "checked":
        return False, diagnostic_reason(checked)

    command = [str(binary), "run", str(source), "--format", "json"]
    if env_path.is_file():
        command.extend(["--mock-env", str(env_path)])
    try:
        run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True,
                             encoding="utf-8", errors="replace", timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        return False, "run: tiempo de espera (10 s)"
    try:
        result = json.loads(run.stdout)
    except json.JSONDecodeError:
        return False, "run: salida JSON inválida o internal_error"
    outcome = result.get("outcome", {})
    if outcome.get("kind") == "driver_error":
        return False, "run: " + str(outcome.get("reason", "driver_error"))
    if outcome.get("kind") not in ("unit", "ok", "err"):
        return False, "run: resultado " + str(outcome.get("kind", "desconocido"))
    if run.returncode != 0 and outcome.get("kind") != "err":
        return False, "run: código de salida inesperado"
    if result.get("diagnostics"):
        return False, "run: diagnóstico de ejecución"
    expected_path = source.with_suffix(".out")
    if expected_path.is_file():
        expected = expected_path.read_text(encoding="utf-8")
        actual = "".join(line + "\n" for line in result.get("stdout", []))
        if actual != expected:
        return False, "run: la salida difiere de .out"
    return True, "?"


def rows(binary: Path) -> list[dict[str, str]]:
    result = []
    for source in sorted(CORPUS.glob("*.n")):
        metadata = source.read_text(encoding="utf-8").splitlines()[0]
        match = metadata.removeprefix("// ").split("; capacidad: ", 1)
        origin = match[0] if match else "procedencia no indicada"
        capability = match[1].rstrip(".") if len(match) > 1 else "capacidad no indicada"
        executes, reason = run_one(binary, source)
        result.append({"id": source.stem[:2], "archivo": source.name,
                       "procedencia": origin, "capacidad": capability,
                       "estado": "EJECUTA" if executes else "NO EJECUTA",
                       "motivo": reason})
    return result


def table(items: list[dict[str, str]]) -> str:
    lines = ["| ID | Archivo | Procedencia | Capacidad | Estado hoy | Motivo |",
             "|---|---|---|---|---|---|"]
    for item in items:
        cells = [item[key].replace("|", "\\|").replace("\n", " ")
                 for key in ("id", "archivo", "procedencia", "capacidad", "estado", "motivo")]
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines)


def update_handoff(count: int, total: int) -> None:
    line = f"Programas objetivo que ejecutan: {count} de {total} (corpus/objetivo/, scripts/objetivo.py)"
    text = HANDOFF.read_text(encoding="utf-8")
    lines = text.splitlines()
    matches = [i for i, value in enumerate(lines) if value.startswith("Programas objetivo que ejecutan:")]
    if matches:
        lines[matches[0]] = line
        for index in reversed(matches[1:]):
            del lines[index]
        updated = "\n".join(lines) + ("\n" if text.endswith(("\n", "\r")) else "")
    else:
        updated = text.rstrip("\r\n") + "\n\n" + line + "\n"
    HANDOFF.write_text(updated, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--actualizar", action="store_true",
                        help="actualiza INDEX.md y la cifra de HANDOFF.md")
    args = parser.parse_args()
    try:
        binary = build_binary()
        items = rows(binary)
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 2
    count = sum(item["estado"] == "EJECUTA" for item in items)
    print(table(items))
    print(f"\nProgramas que ejecutan: {count}/{len(items)}")
    if args.actualizar:
        current = INDEX.read_text(encoding="utf-8") if INDEX.exists() else "# Corpus objetivo\n\n"
        sections = current.split("<!-- tabla-programas -->")
        prefix = sections[0]
        suffix = sections[2] if len(sections) > 2 else ""
        INDEX.write_text(prefix + "<!-- tabla-programas -->\n" + table(items) +
                         "\n<!-- /tabla-programas -->" + suffix,
                         encoding="utf-8", newline="\n")
        update_handoff(count, len(items))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

