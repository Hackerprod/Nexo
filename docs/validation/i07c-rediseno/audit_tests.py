#!/usr/bin/env python3
"""Inventory private Checker calls in the baseline type tests."""

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3]
source = (ROOT / "src/check/types.rs").read_text(encoding="utf-8")
document = (ROOT / "docs/correcciones/I07c-rediseno-restauracion-atomica-R6-final.md").read_text(encoding="utf-8")
listed = set(re.findall(r"^\| `([^`]+)` \|", document, re.M))
starts = list(re.finditer(r"#\[test\]\s*(?:#\[[^\]]+\]\s*)*fn\s+(\w+)\s*\([^)]*\)\s*\{", source))
private = re.compile(r"\b(?:checker|c|ck|left|right|solver)\.(?:union\w*|new_variable|root|require_array|bind\w*|unify\w*|type_nodes|concrete|array_origin|shape_plan|concrete_plan|seed_planned_shape|fact_mut)\b")
print("tests", len(starts), "listed", len(listed))
for i, start in enumerate(starts):
    end = starts[i + 1].start() if i + 1 < len(starts) else len(source)
    name = start.group(1)
    body = source[start.start():end]
    if name not in listed:
        calls = sorted(set(re.findall(r"\b(?:checker|c|ck|left|right|solver)\.(\w+)\s*\(", body)))
        print(name, "PRIVATE" if private.search(body) else "other", ",".join(calls[:20]))
