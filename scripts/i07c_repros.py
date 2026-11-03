#!/usr/bin/env python3
"""Comprueba los repros de tipos conservados y la coherencia check/run."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXE = Path(os.environ.get("NEXO_EXE", str(ROOT / "target/release" /
                                            ("nexo.exe" if os.name == "nt" else "nexo"))))
FOLDERS = ("forms",)
OUTPUT = ROOT / "docs/validation/i07c-repros.json"


def run(command: str, source: Path) -> tuple[int, str]:
    done = subprocess.run([str(EXE), command, str(source)], cwd=ROOT,
                          capture_output=True, text=True, encoding="utf-8")
    output = done.stdout + done.stderr
    assert done.returncode in (0, 1), (source, command, done.returncode, output)
    assert not any(word in output for word in ("internal_error", "InvalidIr", "panicked at")), (source, command, output)
    return done.returncode, output


def diagnostics(output: str) -> list[tuple]:
    entries = []
    current = None
    for line in output.splitlines():
        match = re.match(r"^(NX-E\d{4}):", line)
        if match:
            if current is not None:
                entries.append(current)
            current = {"code": match.group(1), "related": []}
        elif current is not None and line.startswith("  "):
            name, _, value = line.strip().partition(": ")
            if name == "at":
                current["at"] = json.loads(value)
            elif name == "related":
                current["related"].append(json.loads(value))
            elif name == "data":
                current["data"] = json.loads(value)
            elif name == "subcause":
                current["subcause"] = value
    if current is not None:
        entries.append(current)
    return entries


def main() -> None:
    assert EXE.is_file(), EXE
    rows = []
    for folder in FOLDERS:
        files = sorted((ROOT / "tests/fixtures/i07c_repros" / folder).glob("*.n"))
        assert files, folder
        for source in files:
            check_code, check_text = run("check", source)
            run_code, run_text = run("run", source)
            checked = diagnostics(check_text)
            executed = diagnostics(run_text)
            spans = [tuple(d["at"][name] for name in ("start", "end"))
                     for d in checked if "at" in d]
            assert len(spans) == len(set(spans)), (source, checked)
            if checked != executed:
                assert not checked and run_code == 1 and [d["code"] for d in executed] == ["NX-E0112"], (source, checked, executed)
            assert bool(checked) == bool(check_code), (source, check_code, checked)
            rows.append({"file": source.relative_to(ROOT).as_posix(),
                         "source_sha256": hashlib.sha256(source.read_bytes().replace(b"\r\n", b"\n")).hexdigest(),
                         "check_exit": check_code, "run_exit": run_code,
                         "diagnostic_codes": [d["code"] for d in checked],
                         "same_diagnostics": checked == executed})
    OUTPUT.write_text(json.dumps({"programs": len(rows), "rows": rows},
                                 ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print(f"Repros: {len(rows)}; check/run coherentes salvo entrada ausente")


if __name__ == "__main__":
    main()
