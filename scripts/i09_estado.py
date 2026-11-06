#!/usr/bin/env python3
"""Regenerate the I09 summary from versioned evidence."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/validation/i09-evidence"
POPULATION = ROOT / "docs/validation/i09-mutations/population.json"
RESULTS = ROOT / "docs/validation/i09-mutations/results.jsonl"
OUTPUT = ROOT / "docs/validation/F0-0002-I09-estado.md"


def read(name: str) -> str:
    path = EVIDENCE / name
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def suite(name: str) -> str:
    raw = read(name)
    if not raw:
        return "NOT EVALUATED"
    if "test result: FAILED" in raw or "error: test failed" in raw:
        return "FAILED"
    runs = re.findall(r"test result: ok\. (\d+) passed; 0 failed", raw)
    return f"completed; {sum(map(int, runs))} tests passed across packages" if runs else "INCONCLUSIVE"


def campaign(name: str, minimum: int) -> str:
    raw = read(name)
    if not raw:
        return "NOT EVALUATED"
    fields = [re.search(rf"\b{field}=(\d+)", raw) for field in ("seed", "count", "negative")]
    if not all(fields):
        return "INCONCLUSIVE"
    seed, count, negative = (int(value.group(1)) for value in fields)
    if "test result: ok" not in raw or count < minimum:
        return "FAILED"
    return f"completed; seed {seed}; {count} programs; {negative} negative cases"


def mutation_status(population: list[dict]) -> str:
    if not RESULTS.exists():
        return "NOT EVALUATED"
    rows = []
    for line in RESULTS.read_text(encoding="utf-8").splitlines():
        if line.strip():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    profile = "debug" if rows and all("release" not in row.get("result", {}) for row in rows) else "release"
    classified = all(
        row.get("result", {}).get(profile) == "detectada"
        or (row.get("result", {}).get(profile) == "equivalente" and row.get("equivalence"))
        for row in rows
    )
    if len(rows) == len(population) and classified:
        detected = sum(row.get("result", {}).get(profile) == "detectada" for row in rows)
        equivalent = len(rows) - detected
        suffix = f"; {equivalent} documented equivalent" if equivalent else ""
        return f"completed; {detected}/{len(population)} production mutations detected in {profile}{suffix}"
    detected = sum(row.get("result", {}).get(profile) == "detectada" for row in rows)
    survivors = sum(row.get("result", {}).get(profile) == "superviviente" for row in rows)
    return f"in progress; {detected}/{len(population)} detected; {survivors} survivors"


def main() -> None:
    population = json.loads(POPULATION.read_text(encoding="utf-8"))["mutations"]
    rd_values = re.findall(r"RD normative: (\d+)/(\d+) check/run pairs", read("rd.log"))
    rd = ", ".join(f"{left}/{right}" for left, right in rd_values) or "NOT EVALUATED"
    validate_log = read("validate.log")
    validation = "completed" if "i09-validate:ok" in validate_log else ("FAILED" if validate_log else "NOT EVALUATED")
    clippy_log = read("clippy.log")
    clippy = "completed" if "Finished `dev` profile" in clippy_log and "error:" not in clippy_log else ("FAILED" if clippy_log else "NOT EVALUATED")
    fmt = "completed" if "fmt:ok" in read("fmt.log") else "NOT EVALUATED"
    lines = [
        "# F0-0002 I09 - measured status",
        "",
        "Figures come from versioned evidence and the generated mutation population. Independent review and phase approval remain pending.",
        "",
        "| Check | Result |",
        "|---|---|",
        f"| `cargo test --locked --offline` | {suite('debug.log')} |",
        f"| `cargo test --locked --offline --release` | {suite('release.log')} |",
        f"| Normative RD check/run | {rd} |",
        f"| `sh scripts/validate.sh` in an isolated clean copy | {validation} |",
        f"| `cargo clippy --all-targets -- -D warnings` | {clippy} |",
        f"| `cargo fmt --all -- --check` | {fmt} |",
        f"| Typed debug campaign | {campaign('campaign-debug.log', 100_000)} |",
        f"| Typed release campaign | {campaign('campaign-release.log', 300_000)} |",
        f"| Production mutation population | {len(population)} |",
        f"| Real mutation run in a copy | {mutation_status(population)} |",
        "",
        "Implementation notes describe design decisions, quota limits, and ADR gaps.",
        "",
    ]
    OUTPUT.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    print(OUTPUT)


if __name__ == "__main__":
    main()
