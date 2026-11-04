#!/usr/bin/env python3
"""Genera el estado medido de I08 desde registros y la población de producción."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOGS = ROOT / "docs/validation/i08-evidence"
OUTPUT = ROOT / "docs/validation/F0-0002-I08-estado.md"


def content(name: str) -> str:
    path = LOGS / name
    if not path.exists():
        return ""
    raw = path.read_bytes()
    return raw.decode("utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8", errors="replace")


def suite(name: str) -> str:
    text = content(name)
    if not text:
        return "NO EVALUADO"
    if "test result: FAILED" in text or "error: test failed" in text:
        return "FALLA"
    summaries = re.findall(r"test result: ok\. (\d+) passed; 0 failed", text)
    return f"verde; {sum(map(int, summaries))} pruebas activas" if summaries else "INCONCLUSO"


def campaign(name: str) -> str:
    text = content(name)
    fields = [re.search(rf"\b{field}=(\d+)", text) for field in ("seed", "count", "negative")]
    if not all(fields):
        return "NO EVALUADO" if not text else "INCONCLUSO"
    if "test result: ok" not in text:
        return "FALLA"
    seed, count, negative = (int(field.group(1)) for field in fields)
    minimum = 100_000 if "debug" in name else 300_000
    if count < minimum or seed != 9306578997655527858:
        return "FALLA: población o semilla incorrecta"
    return f"verde; semilla {seed}; {count} programas; {negative} negativos"


def mutation() -> str:
    population = ROOT / "docs/validation/i08-mutations/population.json"
    if not population.exists():
        return "NO EVALUADO"
    items = json.loads(population.read_text(encoding="utf-8"))["mutations"]
    result_path = population.parent / "results.jsonl"
    if not result_path.exists():
        return f"{len(items)} mutaciones de producción enumeradas; ejecución NO EVALUADA"
    rows = [json.loads(line) for line in result_path.read_text(encoding="utf-8").splitlines()]
    expected_ids = [item["id"] for item in items]
    if [row["id"] for row in rows] != expected_ids:
        targeted = content("targeted-mutations.log")
        ids = ("M00025-quota", "M00036-entry", "M00037-build", "M00038-branch")
        measured = all(f"{id} detected" in targeted for id in ids)
        detail = "M00025, M00036, M00037 y M00038 dirigidas detectadas" if measured else "dirigidas NO EVALUADAS"
        return (f"{len(items)} mutaciones de producción enumeradas; ejecución completa de población NO EVALUADA "
                f"(resultados versionados corresponden a otra población); {detail}")
    detected = sum(any(v in {"detectada", "no_compila"} for v in row["result"].values()) for row in rows)
    inconclusive = [row["id"] for row in rows if any(v == "inconclusa" for v in row["result"].values())]
    survivors = [row["id"] for row in rows if any(v == "superviviente" for v in row["result"].values())]
    timeouts = []
    for row in rows:
        details = [
            f"{profile}: {row['result'].get(profile, 'sin clasificación')} ({witness})"
            for profile, witness in row.get("witness", {}).items()
            if "no termina" in witness or "plazo" in witness
        ]
        if details:
            timeouts.append(f"{row['id']} ({'; '.join(details)})")
    return (f"{len(items)} enumeradas; {len(rows)} ejecutadas; {detected} detectadas/no compilan; "
            f"supervivientes: {', '.join(survivors) if survivors else 'ninguna'}; "
            f"inconclusas: {', '.join(inconclusive) if inconclusive else 'ninguna'}; "
            f"plazo agotado: {', '.join(timeouts) if timeouts else 'ninguno'}")


def main() -> None:
    rd = content("release.log") + content("debug.log")
    rd_values = re.findall(r"RD normative: (\d+)/(\d+) check/run pairs", rd)
    rd_text = ", ".join(f"{a}/{b}" for a, b in rd_values) if rd_values else "NO EVALUADO"
    validate = content("validate.log")
    validate_text = "verde" if "i08-validate:ok" in validate else ("NO EVALUADO" if not validate else "INCONCLUSO")
    clippy = content("clippy.log")
    clippy_text = "verde" if "Finished `dev` profile" in clippy and "error:" not in clippy else ("NO EVALUADO" if not clippy else "FALLA")
    robustness = content("fuzz.log")
    fuzz_text = "verde" if "test result: ok" in robustness and "0 failed" in robustness else ("NO EVALUADO" if not robustness else "FALLA")
    lines = [
        "# F0-0002 I08 — estado medido",
        "",
        "Resumen de mediciones registradas; revisión independiente y aprobación de F0 pendientes.",
        "",
        "| Comprobación | Resultado |",
        "|---|---|",
        f"| `cargo test --locked --offline` | {suite('debug.log')} |",
        f"| `cargo test --locked --offline --release` | {suite('release.log')} |",
        f"| RD normativo check/run | {rd_text} |",
        f"| `sh scripts/validate.sh` en clon limpio | {validate_text} |",
        f"| `cargo clippy --all-targets -- -D warnings` | {clippy_text} |",
        f"| `cargo fmt --all -- --check` | {'verde' if 'fmt:ok' in content('fmt.log') else 'NO EVALUADO'} |",
        f"| Campaña tipada debug | {campaign('campaign-debug.log')} |",
        f"| Campaña tipada release | {campaign('campaign-release.log')} |",
        f"| Fuzz de solidez Result | {fuzz_text} |",
        f"| Pilas 128/256/512 KiB | {'verde' if 'result_entry_and_propagation_on_small_stacks ... ok' in content('validate.log') else 'NO EVALUADO'} |",
        f"| Mutaciones de producción | {mutation()} |",
        "",
        "Los resultados y límites de los requisitos S09, S20–S21, P10–P11, PP05, R01–R03, F15 y NX-E0112 se explican en las notas de implementación. Este estado no constituye un veredicto de revisión independiente.",
        "",
    ]
    OUTPUT.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    print(OUTPUT)


if __name__ == "__main__":
    main()
