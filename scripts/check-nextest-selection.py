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
SUITE = ROOT / "tests/suite.rs"
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
    if set(fast) != {"inherits", "default-filter", "slow-timeout"} or fast["slow-timeout"] != {"period": "11m", "terminate-after": 1}:
        fail("fast solo puede diferir en filtro y timeout de 11 min, sin reintentos")
    actual = fast["default-filter"]
    expected = "not (" + " | ".join(f"(binary_id(={binary}) & test(={name}))" for binary, name in pairs) + ")"
    if re.sub(r"\s+", "", actual) != re.sub(r"\s+", "", expected):
        fail("el filtro fast no coincide exactamente con los pares de la tabla")


def rust_without_inert_text(source, *, strings=True):
    """Enmascara comentarios anidados y, opcionalmente, cadenas Rust sin mover líneas."""
    out = list(source)
    pos = 0
    while pos < len(source):
        start = pos
        if source.startswith("//", pos):
            end = source.find("\n", pos)
            pos = len(source) if end < 0 else end
        elif source.startswith("/*", pos):
            pos += 2
            depth = 1
            while pos < len(source) and depth:
                if source.startswith("/*", pos):
                    depth += 1
                    pos += 2
                elif source.startswith("*/", pos):
                    depth -= 1
                    pos += 2
                else:
                    pos += 1
        else:
            raw = re.match(r'(?:br|r)(#+)?"', source[pos:])
            if raw:
                suffix = '"' + (raw[1] or "")
                end = source.find(suffix, pos + len(raw[0]))
                pos = len(source) if end < 0 else end + len(suffix)
            elif source[pos] == '"' or source.startswith('b"', pos):
                pos += 2 if source.startswith('b"', pos) else 1
                while pos < len(source):
                    if source[pos] == "\\":
                        pos += 2
                    elif source[pos] == '"':
                        pos += 1
                        break
                    else:
                        pos += 1
            else:
                pos += 1
                continue
            if not strings:
                continue
        for index in range(start, min(pos, len(source))):
            if source[index] != "\n":
                out[index] = " "
    return "".join(out)


def test_sources():
    paths = sorted((ROOT / "tests").glob("*.rs"))
    paths += sorted((ROOT / "tests").glob("*/main.rs"))
    return paths


def suite_sources():
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if manifest["package"].get("autotests") is not False:
        fail("Cargo debe declarar autotests = false")
    targets = {(test.get("name"), test.get("path")) for test in manifest.get("test", [])}
    required = {("suite", "tests/suite.rs"),
                ("i07c_mutant_projection_scale", "tests/i07c_mutant_projection_scale.rs")}
    if not required <= targets or any(not isinstance(name, str) or not isinstance(path, str)
                                      or not path.startswith("tests/") for name, path in targets):
        fail("faltan los destinos suite/asignador o hay un [[test]] inválido")
    content = rust_without_inert_text(SUITE.read_text(encoding="utf-8"), strings=False)
    active = rust_without_inert_text(content)
    declaration_pattern = (r'^#\[path = "([A-Za-z0-9_]+)\.rs"\]\n'
                           r'(?:#\[[^\n]+\]\n)*mod ([A-Za-z0-9_]+);$')
    declarations = [(match[1], match[2]) for match in re.finditer(declaration_pattern, content, re.MULTILINE)
                    if active[match.start():].startswith("#[path")]
    if any(source != module for source, module in declarations):
        fail("la declaración #[path] debe conservar el nombre de su fuente")
    ordinary = [source for source, _ in declarations]
    sources = {path.stem for path in (ROOT / "tests").glob("*.rs")}
    special = {"suite", "j14_frontend_stack", "j16_frontend_stack",
               "i07c_mutant_projection_scale"}
    separate = {Path(path).stem for _, path in targets if path.endswith(".rs")}
    if len(ordinary) != len(set(ordinary)) or set(ordinary) != sources - special - separate:
        fail("suite.rs omite o duplica una fuente de tests")
    if sources & special != special:
        fail("falta una de las fuentes especiales de tests")
    includes = [match[1] for match in re.finditer(r'include!\("([A-Za-z0-9_]+)\.rs"\);', content)
                if active[match.start():].startswith("include!")]
    if set(includes) != {
            "j14_frontend_stack", "j16_frontend_stack"}:
        fail("J14 y J16 deben conservar sus inclusiones especiales")
    if active.count("#[test]\nfn measured_stack_child()") != 1:
        fail("falta el único alias raíz de measured_stack_child")
    common = {path.stem for path in (ROOT / "tests").glob("*.rs")
              if re.search(r"(?m)^mod common;", path.read_text(encoding="utf-8"))}
    if not common <= set(ordinary) | {"j14_frontend_stack", "j16_frontend_stack"}:
        fail("una fuente con mod common quedó fuera de suite")
    nested = [path.relative_to(ROOT).as_posix() for path in test_sources()
              if path.name == "main.rs" and path.parent.name != "common"
              and path.relative_to(ROOT).as_posix() not in {target for _, target in targets}]
    if nested:
        fail(f"fuente tests/<x>/main.rs sin destino [[test]]: {nested}")
    print(f"suite: {len(ordinary)} módulos ordinarios, {len(common)} con mod common; J14/J16 y asignador aparte")


def verify_source_inventory(all_cases, build):
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    separate = {test["path"]: test["name"] for test in manifest.get("test", [])}
    for path in test_sources():
        relative = path.relative_to(ROOT).as_posix()
        if path == SUITE or relative.startswith("tests/common/"):
            continue
        source = rust_without_inert_text(path.read_text(encoding="utf-8"))
        count = len(re.findall(r"#\s*\[\s*test\s*\]", source))
        if count == 0:
            continue
        if relative in separate:
            binary, prefix = "nexo::" + separate[relative], ""
        elif path.stem == "j16_frontend_stack":
            binary, prefix = "nexo::suite", ""
        else:
            binary, prefix = "nexo::suite", path.stem + "::"
        if path.stem == "j16_frontend_stack":
            # J16 se incluye en la raíz: identificar sus funciones, no todo suite.
            names = re.findall(r"#\s*\[\s*test\s*\]\s*(?:pub\s+)?fn\s+(\w+)", source)
            listed = sum(case_binary == binary and name in names for case_binary, name in all_cases)
        else:
            listed = sum(case_binary == binary and name.startswith(prefix)
                         for case_binary, name in all_cases)
        if listed == 0:
            fail(f"{relative}: ningún test en nextest {build} ({binary}/{prefix}); "
                 "posible archivo no registrado, cfg anulador, #[path] cruzado o declaración comentada")
        if listed != count:
            print(f"AVISO {relative} {build}: {count} atributos #[test], {listed} tests listados "
                  "(cfg legítimo o pruebas generadas); solo cero es fatal")


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


def cargo_test_inventory(build):
    """Inventario del runner clásico cuando nextest no está instalado."""
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    cases = set()
    for target in manifest.get("test", []):
        command = ["cargo", "test", "--locked", "--offline", "--test", target["name"]]
        if build == "release":
            command.append("--release")
        command += ["--", "--list"]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True,
                                encoding="utf-8", errors="replace")
        if result.returncode:
            fail(f"cargo test --list {build}/{target['name']} terminó {result.returncode}: "
                 f"{result.stderr[-1000:]}")
        for line in result.stdout.splitlines():
            if line.endswith(": test"):
                pair = ("nexo::" + target["name"], line[:-len(": test")])
                if pair in cases:
                    fail(f"test duplicado en cargo test --list {build}: {pair}")
                cases.add(pair)
    return cases


def check():
    suite_sources()
    if sys.argv[1:] == ["--cargo-test"]:
        for build in ("debug", "release"):
            verify_source_inventory(cargo_test_inventory(build), build)
        print("selección de fuentes comprobada con cargo test --list (nextest no instalado)")
        return
    if len(sys.argv) != 1:
        fail("uso: check-nextest-selection.py [--cargo-test]")
    pairs = table_pairs()
    filter_pairs(pairs)
    heavy = set(pairs)
    for build in ("debug", "release"):
        all_default, ignored_default, universe = inventory(build, "default")
        verify_source_inventory(all_default, build)
        all_fast, ignored_fast, fast = inventory(build, "fast")
        if all_default != all_fast or ignored_default != ignored_fast:
            fail(f"descubrimiento o ignorados difieren entre default y fast en {build}")
        if universe != all_default - ignored_default:
            fail(f"default deja fuera ejecutables en {build}")
        alias = ("nexo::suite", "measured_stack_child")
        original = ("nexo::suite", "j14_frontend_stack::measured_stack_child")
        children = {("nexo::suite", "parser_child"),
                    ("nexo::suite", "deep_match_type_stack_child")}
        if alias not in universe or original not in universe or not children <= universe:
            fail(f"hijos exactos J14/J16 o alias ausentes en {build}")
        missing = heavy - universe
        if missing:
            fail(f"pares pesados ausentes del inventario ejecutable {build}: {sorted(missing)[:5]}")
        if fast & heavy:
            fail(f"fast ejecuta pares pesados en {build}: {sorted(fast & heavy)[:5]}")
        if fast | heavy != universe or universe - fast != heavy:
            fail(f"fast ∪ pesados ≠ completo en {build}; faltan={sorted(universe - (fast | heavy))[:5]} extras={sorted((fast | heavy) - universe)[:5]}")
        print(f"{build}: completo={len(universe)} (originales={len(universe)-1}, alias=1) "
              f"fast={len(fast)} pesados={len(heavy)} ignorados={len(ignored_default)}")


if __name__ == "__main__":
    try:
        check()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"ERROR selección nextest: {error}", file=sys.stderr)
        raise SystemExit(1)
