#!/usr/bin/env python3
"""Run the literal F0-0003 §9.1/§9.3 pairs through the public CLI."""
from __future__ import annotations

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ADR = ROOT / "docs/adr/F0-0003-third-slice.md"
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
BINARY = TARGET / "release" / ("nexo.exe" if os.name == "nt" else "nexo")


def fixtures() -> list[tuple[str, str, str, str, tuple[int, int] | None]]:
    text = ADR.read_text(encoding="utf-8")
    first = text.split("### 9.1. Fuentes y expectativas exactas", 1)[1].split("### 9.2.", 1)[0]
    third = text.split("### 9.3. Guardas, ciclos y operaciones: fuentes literales", 1)[1].split("### 9.4.", 1)[0]
    negatives = {}
    positives = {}
    for line in first.splitlines():
        match = re.fullmatch(r"\| ([A-Z]\d\d) \| `([^`]+)` \| (NX-E\d+(?:/[a-z_]+)?) \| \[(\d+),(\d+)\) \|", line)
        if match:
            key, source, rule, start, end = match.groups()
            negatives[key] = (source, rule, (int(start), int(end)))
            continue
        match = re.fullmatch(r"\| ([A-Z]\d\d) \| `([^`]+)` \|", line)
        if match:
            positives[match.group(1)] = match.group(2)
    rows = [(key, source, positives[key], rule, span)
            for key, (source, rule, span) in negatives.items() if key in positives]
    for line in third.splitlines():
        match = re.fullmatch(r"\| ([A-Z]\d\d) \| `([^`]+)` \| (NX-E\d+(?:/[a-z_]+)?) \| \[(\d+),(\d+)\) \| `([^`]+)` \|", line)
        if match:
            key, negative, rule, start, end, positive = match.groups()
            rows.append((key, negative, positive, rule, (int(start), int(end))))
    if len(rows) < 30:
        raise RuntimeError(f"too few literal pairs: {len(rows)}")
    return rows


def invoke(path: Path, command: str) -> tuple[int, dict]:
    output = subprocess.run(
        [str(BINARY), command, str(path), "--profile", "f0-0003-b-v1", "--format", "json"],
        cwd=ROOT, capture_output=True, timeout=10, check=False,
    )
    return output.returncode, json.loads(output.stdout)


def main() -> None:
    failures = []
    rows = fixtures()
    with tempfile.TemporaryDirectory(prefix="nexo-j14-fixtures-", dir="D:/") as temporary:
        folder = Path(temporary)
        for key, negative, positive, rule, span in rows:
            for kind, source in (("negative", negative), ("positive", positive)):
                path = folder / f"{key}-{kind}.n"
                path.write_text(source, encoding="utf-8", newline="")
                for command in ("check", "run"):
                    try:
                        code, report = invoke(path, command)
                    except (subprocess.TimeoutExpired, json.JSONDecodeError) as error:
                        failures.append((key, kind, command, type(error).__name__))
                        continue
                    diagnostics = report.get("diagnostics")
                    if kind == "positive":
                        if code != 0 or diagnostics != []:
                            failures.append((key, kind, command, code, diagnostics))
                    else:
                        expected_code, _, expected_suffix = rule.partition("/")
                        actual = diagnostics[0] if isinstance(diagnostics, list) and len(diagnostics) == 1 else {}
                        actual_span = actual.get("primary_span") or {}
                        suffix_ok = (not expected_suffix or
                                     actual.get("subcause") == expected_suffix or
                                     actual.get("feature_id") == expected_suffix)
                        if (code != 1 or len(diagnostics or []) != 1 or
                                actual.get("code") != expected_code or not suffix_ok or
                                (actual_span.get("start"), actual_span.get("end")) != span):
                            failures.append((key, kind, command, code, diagnostics))
    print(json.dumps({"pairs": len(rows), "invocations": len(rows) * 4,
                      "failures": len(failures), "first": failures[:8]}, ensure_ascii=False))
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
