"""Whitelist audit of the oracle's independence (class ORQUESTA; plan 1.5).

Static part: module classes, import graph, forbidden names and calls in MODELO modules,
vetoed path patterns over string tokens, fixture shape and pins.  The dynamic part
(audit hooks) lives in ``selftest.run_audited``.  ``audit`` itself is exempt from the
path veto because it must spell the vetoed patterns.
"""

from __future__ import annotations

import ast
import hashlib
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
FIXTURES = os.path.join(ROOT, "tests", "fixtures")

MODELO = {"utf8ref", "model", "literal", "diagnostics", "generate", "barrier", "app", "charges"}
LECTOR = {"fixtures_io"}
ARNES = {"generic", "runner", "batch"}
ORQUESTA = {"cli", "selftest", "mutate", "audit", "minimize"}
KNOWN = MODELO | LECTOR | ARNES | ORQUESTA | {"__init__"}

MODEL_STDLIB = {"__future__", "dataclasses", "enum", "typing", "random", "hashlib", "itertools",
                "functools", "collections", "fractions"}
FORBIDDEN_IN_MODEL = {"os", "pathlib", "importlib", "runpy", "shutil", "glob", "subprocess", "sys",
                      "tempfile", "socket", "ctypes", "builtins", "json"}
FORBIDDEN_CALLS = {"open", "exec", "eval", "compile", "__import__", "getattr", "globals"}

VETO = [
    re.compile(r"(?<![A-Za-z0-9_])src[/\\]"),
    re.compile(r"corpus/objetivo"),
    re.compile(r"docs/validation"),
    re.compile(r"\.scratch"),
    re.compile(r"F4-r1"),
    re.compile(r"F4-lagunas"),
    re.compile(r"scripts/f5_oracle"),
    re.compile(r"(?<![A-Za-z0-9_])target[/\\]"),
    re.compile(r"CARGO_TARGET_DIR"),
]

# Symbols of the F5 oracle that must never appear in generic.py (plan 1.4).
F5_SYMBOLS = ("projection", "SourceVM", "requirement_sites", "f0-0005", "F5-")


def module_files() -> list[str]:
    return sorted(f for f in os.listdir(HERE) if f.endswith(".py"))


def _strings(tree):
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            yield node.value
        elif isinstance(node, ast.JoinedStr):
            for part in node.values:
                if isinstance(part, ast.Constant) and isinstance(part.value, str):
                    yield part.value


def _imports(tree, name):
    """Yield (target, kind) with kind 'pkg' for in-package modules, 'std' otherwise."""
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                top = alias.name.split(".")[0]
                if alias.name.startswith("scripts.f4_oracle."):
                    yield alias.name.split(".")[2], "pkg"
                else:
                    yield top, "std"
        elif isinstance(node, ast.ImportFrom):
            if node.level >= 1:
                if node.module:
                    yield node.module.split(".")[0], "pkg"
                else:
                    for alias in node.names:
                        yield alias.name, "pkg"
            elif node.module:
                if node.module.startswith("scripts.f4_oracle"):
                    parts = node.module.split(".")
                    if len(parts) > 2:
                        yield parts[2], "pkg"
                    else:
                        for alias in node.names:
                            yield alias.name, "pkg"
                else:
                    yield node.module.split(".")[0], "std"


def _class_of(mod):
    if mod in MODELO:
        return "MODELO"
    if mod in LECTOR:
        return "LECTOR"
    if mod in ARNES:
        return "ARNES"
    if mod in ORQUESTA:
        return "ORQUESTA"
    return None


def static_findings() -> list[str]:
    out: list[str] = []
    for fname in module_files():
        mod = fname[:-3]
        cls = _class_of(mod)
        if mod != "__init__" and cls is None:
            out.append(f"{fname}: module has no class in the audit table")
            continue
        with open(os.path.join(HERE, fname), "rb") as handle:
            source = handle.read().decode("utf-8")
        tree = ast.parse(source)
        if mod != "audit":
            for text in _strings(tree):
                for pat in VETO:
                    if pat.search(text):
                        out.append(f"{fname}: vetoed path pattern {pat.pattern!r} in a string token")
        if cls == "MODELO":
            for target, kind in _imports(tree, mod):
                if kind == "std":
                    if target in FORBIDDEN_IN_MODEL or target not in MODEL_STDLIB:
                        out.append(f"{fname}: MODELO imports non-whitelisted module {target}")
                else:
                    tcls = _class_of(target)
                    if tcls not in ("MODELO", "LECTOR"):
                        out.append(f"{fname}: MODELO imports {target} ({tcls})")
            for node in ast.walk(tree):
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) \
                        and node.func.id in FORBIDDEN_CALLS:
                    out.append(f"{fname}: MODELO calls {node.func.id}")
        if cls == "ARNES":
            for target, kind in _imports(tree, mod):
                if kind == "pkg" and _class_of(target) == "MODELO":
                    out.append(f"{fname}: ARNES imports MODELO module {target}")
            if mod == "generic":
                for text in _strings(tree):
                    for sym in F5_SYMBOLS:
                        if sym in text:
                            out.append(f"generic.py: F5 symbol {sym!r}")
    return out


def _walk_keys(value, path, found):
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "observed" and not (path and path[-1] == "data"):
                found.append("/".join(path + [key]))
            _walk_keys(item, path + [key], found)
    elif isinstance(value, list):
        for i, item in enumerate(value):
            _walk_keys(item, path + [str(i)], found)


def fixture_findings() -> list[str]:
    out: list[str] = []
    if not os.path.isdir(FIXTURES):
        return ["tests/fixtures is missing"]
    for fname in sorted(os.listdir(FIXTURES)):
        if not fname.startswith("f4_oracle_"):
            continue
        path = os.path.join(FIXTURES, fname)
        if fname.endswith(".json"):
            with open(path, "rb") as handle:
                data = json.loads(handle.read().decode("utf-8"))
            found: list = []
            _walk_keys(data, [], found)
            out += [f"{fname}: field observed outside data at {p}" for p in found]
        elif fname.endswith(".jsonl"):
            with open(path, "rb") as handle:
                for n, line in enumerate(handle.read().decode("utf-8").split("\n"), 1):
                    if line.strip():
                        found = []
                        _walk_keys(json.loads(line), [], found)
                        out += [f"{fname}:{n}: field observed at {p}" for p in found]
    return out


def pin_findings() -> list[str]:
    out: list[str] = []
    path = os.path.join(FIXTURES, "f4_oracle_pins.json")
    with open(path, "rb") as handle:
        pins = json.loads(handle.read().decode("utf-8"))
    if pins.get("generic"):
        gpath = os.path.join(HERE, "generic.py")
        if not os.path.exists(gpath):
            out.append("pins list generic symbols but generic.py is missing")
    elif os.path.exists(os.path.join(HERE, "generic.py")):
        out.append("generic.py exists without pins")
    return out


def run() -> list[str]:
    return static_findings() + fixture_findings() + pin_findings()


def sha256_file(path: str) -> str:
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()
