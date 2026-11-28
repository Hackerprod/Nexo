#!/usr/bin/env python3
"""Summarize the frozen C2 commit bisection with row-level attribution."""

import argparse
import collections
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
MECHANISMS = HERE.parents[1] / "correcciones" / "I07c-rediseno-c2-d12-mecanismos.md"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raw = args.input.read_bytes().replace(b"\r\n", b"\n")
    report = json.loads(raw)
    groups = {}
    for name, body in re.findall(r"Miembros (M\d):\s*(.*?)(?=\n\n|\Z)",
                                 MECHANISMS.read_text(encoding="utf-8"), re.S):
        for seed in re.findall(r"\d+", body):
            groups[int(seed)] = name
    assert len(groups) == 62, len(groups)
    rows = report["rows"]
    defects = [(seed, rows[f"fuzz:{seed}"]) for seed in sorted(groups)]
    assert all(row["first_defect"] for _, row in defects)
    improvements = [(int(key.split(":")[1]), row) for key, row in rows.items()
                    if key.startswith("fuzz:") and row["final_status"] == "improvement"]
    assert len(improvements) >= 10 and all(row["first_final_output"] for _, row in improvements)
    counts = collections.Counter(row["first_defect"][:8] for _, row in defects)
    historical = collections.Counter(row["first_difference"][:8] for key, row in rows.items()
                                     if key.startswith("historical:") and row["first_difference"])
    lines = ["# Bisección C2 de defectos D12 — en validación, sin PASS", "",
             f"Base `{report['base']}`; extremo `{report['head']}`; "
             f"{len(report['commits'])}/{report['source_commit_count']} compilaciones de código fuente.",
             f"SHA-256 del registro JSON: `{hashlib.sha256(raw).hexdigest()}`.", "",
             "## Primer introductor de los 62 defectos finales", "",
             "| Commit | M1 | M2 | M3 | M4 | M5 | Total |", "|---|---:|---:|---:|---:|---:|---:|"]
    by_commit = collections.defaultdict(lambda: collections.Counter())
    for seed, row in defects:
        by_commit[row["first_defect"][:8]][groups[seed]] += 1
    ordered = [commit["commit"][:8] for commit in report["commits"]]
    for commit in ordered:
        if commit not in by_commit:
            continue
        cell = by_commit[commit]
        lines.append("| `" + commit + "` | " + " | ".join(str(cell[f"M{i}"]) for i in range(1, 6))
                     + f" | {sum(cell.values())} |")
    lines += ["", "| Grupo | Semilla → primer commit |", "|---|---|"]
    for group in (f"M{i}" for i in range(1, 6)):
        entries = [f"{seed}→`{row['first_defect'][:8]}`" for seed, row in defects if groups[seed] == group]
        lines.append(f"| {group} | {', '.join(entries)} |")
    lines += ["", "## Diez mejoras seleccionadas", "",
              "| Semilla | Primer cambio | Primera salida final observada |",
              "|---:|---|---|"]
    for seed, row in sorted(improvements):
        lines.append(f"| {seed} | `{row['first_difference'][:8]}` | `{row['first_final_output'][:8]}` |")
    lines += ["", "## Diferencias históricas", "",
              "| Primer commit | Filas de 120 |", "|---|---:|"]
    for commit in ordered:
        if historical[commit]:
            lines.append(f"| `{commit}` | {historical[commit]} |")
    lines += ["", "## Censo por compilación", "",
              "| Commit | D12 defectos en 72 | Mejoras en 72 | Diferencias históricas de 120 |",
              "|---|---:|---:|---:|"]
    for entry in report["commits"]:
        cell = entry["counts"]
        lines.append(f"| `{entry['commit'][:8]}` | {cell['d12_defects']} | "
                     f"{cell['d12_improvements']} | {cell['historical_differences']} |")
    args.output.write_bytes(("\n".join(lines) + "\n").encode("utf-8"))
    print("defects", dict(counts), "improvements", len(improvements),
          "historical", sum(historical.values()))


if __name__ == "__main__":
    main()
