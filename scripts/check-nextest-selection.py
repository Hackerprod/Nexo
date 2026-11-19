#!/usr/bin/env python3
"""Comprueba que el filtro fast particiona los ejecutables de nextest."""

import sys

if sys.version_info < (3, 11):
    print("ERROR: instale Python 3.11+ (tomllib) para comprobar la selección de nextest", file=sys.stderr)
    raise SystemExit(2)

import json
import os
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "docs/validation/NX-build-mediciones.md"
CONFIG = ROOT / ".config/nextest.toml"
START = "<!-- NX-BUILD-EXCLUSIONS-START -->"
END = "<!-- NX-BUILD-EXCLUSIONS-END -->"


def fail(message):
    raise ValueError(message)


def table_pairs():
    content = TABLE.read_text(encoding="utf-8")
    if content.count(START) != 1 or content.count(END) != 1:
        fail("faltan los marcadores únicos de la tabla de pesados")
    section = content.split(START, 1)[1].split(END, 1)[0]
    lines = [line.strip() for line in section.splitlines() if line.strip()]
    if not lines or not lines[0].startswith("| binary_id | test_name |"):
        fail("cabecera de tabla de pesados inválida")
    pairs = []
    seen = set()
    for line in lines[2:]:
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 6:
            fail(f"fila de pesados inválida: {line}")
        binary, name = cells[:2]
        if not (binary.startswith("`") and binary.endswith("`") and name.startswith("`") and name.endswith("`")):
            fail(f"binary_id/test_name deben ser literales entre acentos graves: {line}")
        pair = binary[1:-1], name[1:-1]
        if not all(re.fullmatch(r"[A-Za-z0-9_:/.-]+", value) for value in pair):
            fail(f"par con caracteres no literales: {pair}")
        if pair in seen:
            fail(f"par duplicado en la tabla: {pair}")
        seen.add(pair)
        pairs.append(pair)
    if not pairs:
        fail("tabla de pesados vacía")
    return pairs


def filter_pairs(pairs):
    data = tomllib.loads(CONFIG.read_text(encoding="utf-8"))
    profiles = data.get("profile", {})
    default = profiles.get("default")
    fast = profiles.get("fast")
    if not isinstance(default, dict) or "default-filter" in default:
        fail("[profile.default] debe existir y no restringir pruebas")
    if not isinstance(fast, dict) or fast.get("inherits") != "default":
        fail("[profile.fast] debe heredar de default")
    if set(default) != {"slow-timeout"} or default["slow-timeout"] != {"period": "30m", "terminate-after": 1}:
        fail("default debe conservar el timeout completo de 30 min, sin reintentos")
    if set(fast) != {"inherits", "default-filter", "slow-timeout"} or fast["slow-timeout"] != {"period": "8m", "terminate-after": 1}:
        fail("fast solo puede diferir en filtro y timeout de 8 min, sin reintentos")
    actual = fast["default-filter"]
    expected = "not (" + " | ".join(f"(binary_id(={binary}) & test(={name}))" for binary, name in pairs) + ")"
    if re.sub(r"\s+", "", actual) != re.sub(r"\s+", "", expected):
        fail("el filtro fast no coincide exactamente con los pares de la tabla")


def inventory(build, profile):
    command = ["cargo", "nextest", "list", "--locked", "--offline"]
    if build == "release":
        command.append("--release")
    command += ["--profile", profile, "--message-format", "json"]
    env = os.environ.copy()
    env["NEXTEST_PROFILE"] = "fast" if profile == "default" else "default"
    result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if result.returncode != 0:
        fail(f"nextest list {build}/{profile} terminó {result.returncode}: {result.stderr[-1000:]}")
    data = json.loads(result.stdout)
    all_cases = set()
    ignored = set()
    selected = set()
    for binary, suite in data["rust-suites"].items():
        for name, case in suite["testcases"].items():
            pair = (binary, name)
            if pair in all_cases:
                fail(f"par duplicado en inventario {build}/{profile}: {pair}")
            all_cases.add(pair)
            if case["ignored"]:
                ignored.add(pair)
            elif case["filter-match"]["status"] == "matches":
                selected.add(pair)
    if len(all_cases) != data["test-count"]:
        fail(f"cardinalidad descubierta incoherente en {build}/{profile}")
    return all_cases, ignored, selected


def check():
    pairs = table_pairs()
    filter_pairs(pairs)
    heavy = set(pairs)
    for build in ("debug", "release"):
        all_default, ignored_default, universe = inventory(build, "default")
        all_fast, ignored_fast, fast = inventory(build, "fast")
        if all_default != all_fast or ignored_default != ignored_fast:
            fail(f"descubrimiento o ignorados difieren entre default y fast en {build}")
        if universe != all_default - ignored_default:
            fail(f"default deja fuera ejecutables en {build}")
        missing = heavy - universe
        if missing:
            fail(f"pares pesados ausentes del inventario ejecutable {build}: {sorted(missing)[:5]}")
        if fast & heavy:
            fail(f"fast ejecuta pares pesados en {build}: {sorted(fast & heavy)[:5]}")
        if fast | heavy != universe or universe - fast != heavy:
            fail(f"fast ∪ pesados ≠ completo en {build}; faltan={sorted(universe - (fast | heavy))[:5]} extras={sorted((fast | heavy) - universe)[:5]}")
        print(f"{build}: completo={len(universe)} fast={len(fast)} pesados={len(heavy)} ignorados={len(ignored_default)}")


if __name__ == "__main__":
    try:
        check()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"ERROR selección nextest: {error}", file=sys.stderr)
        raise SystemExit(1)
