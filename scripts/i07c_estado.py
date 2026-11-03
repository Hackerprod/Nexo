#!/usr/bin/env python3
"""Regenera el bloque medido de I07c desde resultados versionados."""

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

sys.dont_write_bytecode = True
from i07c_mutaciones import candidates, is_test_only

ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "docs/validation"
MUTATIONS = VALIDATION / "i07c-mutations"
OUTPUT = VALIDATION / "F0-0002-I07c-estado.md"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="b506d7d")
    args = parser.parse_args()
    population = read_json(MUTATIONS / "population.json")
    entries = population["mutations"]
    assert population["base"] == args.base
    assert entries == candidates(args.base), "población desfasada"
    assert all(not is_test_only(item) for item in entries)
    by_id = {item["id"]: item for item in entries}
    rows = [json.loads(line) for line in
            (MUTATIONS / "results.jsonl").read_text(encoding="utf-8").splitlines()]
    assert len(rows) == len(entries) == len({row["id"] for row in rows})
    assert [row["id"] for row in rows] == [item["id"] for item in entries]
    for row in rows:
        fingerprint = hashlib.sha256(json.dumps(by_id[row["id"]], sort_keys=True).encode()).hexdigest()
        assert row["mutation_sha256"] == fingerprint, row["id"]
        assert row["result"]["release"] in {"detectada", "no_compila", "superviviente"}, row["id"]
        if row["result"]["release"] == "detectada":
            assert row["witness"].get("release"), row["id"]
    informative = read_json(MUTATIONS / "informative.json")
    equivalent = read_json(MUTATIONS / "equivalence.json")
    i10a = read_json(MUTATIONS / "i10a.json")
    survivors = {row["id"] for row in rows if row["result"]["release"] == "superviviente"}
    assigned = set(informative) | set(equivalent) | set(i10a)
    assert survivors == assigned and len(informative) + len(equivalent) + len(i10a) == len(assigned)
    assert all(isinstance(argument, str) and argument.strip()
               for argument in list(equivalent.values()) + list(i10a.values()))
    verified = [json.loads(line) for line in
                (MUTATIONS / "detected-verification.jsonl").read_text(encoding="utf-8").splitlines()]
    by_verified = {item["id"]: item for item in verified}
    detected = {row["id"] for row in rows if row["result"]["release"] == "detectada"}
    assert len(verified) == len(by_verified) and set(by_verified) == detected
    for id, item in by_verified.items():
        assert item["status"] in {"detected", "detected_timeout"}, id
        assert item["test"] and item["test"] in next(row for row in rows if row["id"] == id)["witness"]["release"]
        assert item["source_sha256"] == hashlib.sha256((ROOT / by_id[id]["file"]).read_bytes()).hexdigest()
        assert item["status"] != "detected_timeout" or item["baseline_seconds"] < 10
    hashes = read_json(MUTATIONS / "source-sha256.json")
    assert hashes["base"] == args.base
    assert {item["file"] for item in entries} == {row["file"] for row in hashes["files"]}
    for row in hashes["files"]:
        digest = hashlib.sha256((ROOT / row["file"]).read_bytes().replace(b"\r\n", b"\n")).hexdigest()
        assert digest == row["before"] == row["after"] == row["copy_after"]

    suites = read_json(VALIDATION / "i07c-suite-results.json")["profiles"]
    assert set(suites) == {"debug", "release"}
    for suite in suites.values():
        assert suite["exit_code"] == 0
        assert suite["rd"] == "152/152 check/run pairs normativos"
    ignored = read_json(VALIDATION / "i07c-ignored-results.json")["runs"]
    assert [run["threads"] for run in ignored] == [4, 1]
    assert ignored[0]["tests"] == ignored[1]["tests"]
    assert ignored[0]["campaign_programs"] == ignored[1]["campaign_programs"]
    assert all(value > 0 for value in ignored[0]["campaign_programs"].values())
    assert all(run["failed"] == 0 and run["passed"] == len(run["tests"]) for run in ignored)
    for run in ignored:
        source = (ROOT / "src/check/types.rs").read_bytes().replace(b"\r\n", b"\n")
        digest = hashlib.sha256(source + "\n".join(run["tests"]).encode()).hexdigest()
        assert run["input_sha256"] == digest
        assert 0 < run["max_scale_ratio"] <= 4.6
        assert run["max_clock_ratio"] > 0
        assert all(name in run["tests"] for name in run["informative_tests"])
    validation = read_json(VALIDATION / "i07c-validate.json")
    assert validation["command"] == "sh scripts/validate.sh"
    assert validation["clean_copy_without_scratch"] is True
    assert validation["fresh_target"] is True
    assert validation["exit_code"] == 0

    campaigns = read_json(VALIDATION / "i07c-campaign-results.json")
    for item in campaigns:
        result = read_json(ROOT / item["result_file"])
        assert result["programs"] == item["programs"]
        assert result["checks"] == item["checks"]
        assert result["informative"] == item["informative"] == (item["name"] in {"quota_reduced_release", "deferred_release"})
    deferred = read_json(VALIDATION / "i07c-campaign-results/deferred_release.json")["summary"]
    twins = read_json(VALIDATION / "i07c-gemelos.json")
    typed = read_json(VALIDATION / "i07c-typed-campaign-results.json")
    typed_source = (ROOT / "src/check/types.rs").read_bytes().replace(b"\r\n", b"\n")
    domain = read_json(VALIDATION / "i07c-domain-results.json")
    assert domain["source_sha256"] == hashlib.sha256(typed_source).hexdigest()
    assert {row["profile"] for row in domain["campaigns"]} == {"debug", "release"}
    for profile in ("debug", "release"):
        measured = [row for row in domain["campaigns"] if row["profile"] == profile]
        assert len(measured) == 4
        assert sum(row["programs"] for row in measured) >= 300_000
        assert sum(row["comparable"] for row in measured) >= 300_000
        assert all(row["bugs"] == row["duplicates"] == 0 for row in measured)
    assert typed["source_sha256"] == hashlib.sha256(typed_source).hexdigest()
    assert typed["adjudication_sha256"] == hashlib.sha256(
        (VALIDATION / "i07c-typed-exceptions.json").read_bytes()
    ).hexdigest()
    assert {row["profile"] for row in typed["campaigns"]} == {"debug", "release"}
    assert all(row["programs"] >= 300_000
               and row["reduced_signatures"] == row["adjudicated"]
               and row["bugs"] == row["duplicates"] == 0
               for row in typed["campaigns"])
    normative = read_json(MUTATIONS / "normative-classification.json")
    assert normative["projection_mode"] == "projection_full"
    assert normative["survivors_measured"] == len(survivors)
    assert normative["informative"] == len(informative)
    assert normative["equivalent"] == len(equivalent)
    assert normative["survivor_normative_differences"] == 0
    assert len(twins["exemption_families"]) == 4
    classes = Counter(row["result"]["release"] for row in rows)
    library = {profile: suites[profile]["binaries"]["src/lib.rs"]
               for profile in ("debug", "release")}
    integration = {profile: sum(v["passed"] for k, v in suites[profile]["binaries"].items()
                                if k not in ("src/lib.rs", "src/main.rs"))
                   for profile in ("debug", "release")}
    campaign_table = "\n".join(
        f'| {item["name"]} | {item["programs"]} | {item["checks"]} | {"informativo" if item["informative"] else "puerta"} |'
        for item in campaigns
    )
    typed_table = "\n".join(
        f'| {row["profile"]} | {row["programs"]} | {row["reduced_signatures"]} | {row["adjudicated"]} |'
        for row in typed["campaigns"]
    )
    domain_table = "\n".join(
        f'| {profile} | {sum(row["programs"] for row in domain["campaigns"] if row["profile"] == profile)} | '
        f'{sum(row["comparable"] for row in domain["campaigns"] if row["profile"] == profile)} |'
        for profile in ("debug", "release")
    )
    content = f"""# Estado medido de I07c

Generado con `python scripts/i07c_estado.py` desde resultados versionados y la referencia `b506d7d`.

| Magnitud | Debug | Release |
|---|---:|---:|
| Biblioteca activa | {library['debug']['passed']} | {library['release']['passed']} |
| Biblioteca ignorada | {library['debug']['ignored']} | {library['release']['ignored']} |
| Integración activa | {integration['debug']} | {integration['release']} |
| RD normativo | {suites['debug']['rd']} | {suites['release']['rd']} |

`sh scripts/validate.sh` terminó con código {validation['exit_code']} en copia limpia del árbol y con target nuevo.

| Pruebas ignoradas en release | Completadas | Fallos |
|---:|---:|---:|
| {ignored[0]['threads']} hilos | {ignored[0]['passed']} | {ignored[0]['failed']} |
| {ignored[1]['threads']} hilo | {ignored[1]['passed']} | {ignored[1]['failed']} |

La biblioteca tiene {library['debug']['ignored']} pruebas ignoradas; la escala de proyección, el oráculo RHS y los gemelos extensos son tres integraciones adicionales. Por eso cada campaña completa {ignored[0]['passed']} pruebas. El oráculo RHS ejecuta {ignored[0]['campaign_programs']['rhs']} fuentes y los gemelos extensos {ignored[0]['campaign_programs']['twins']}, calculados desde sus bucles fuente y registrados en ambos modos de hilos.

| Razón máxima de escala | 4 hilos | 1 hilo |
|---|---:|---:|
| Pasos y asignaciones | {ignored[0]['max_scale_ratio']:.3f} | {ignored[1]['max_scale_ratio']:.3f} |
| Tiempo de reloj (informativo) | {ignored[0]['max_clock_ratio']:.3f} | {ignored[1]['max_clock_ratio']:.3f} |

El límite 4,6 se aplica a pasos y asignaciones; la duración no forma parte de esa puerta.

| Población de producción | Release |
|---|---:|
| Candidatas | {len(entries)} |
| Detectadas | {classes['detectada']} |
| Citas detectadas revalidadas | {len(verified)} |
| No compilan | {classes['no_compila']} |
| Supervivientes | {classes['superviviente']} |
| Supervivientes informativos | {len(informative)} |
| Equivalencias normativas verificadas | {len(equivalent)} |
| Deuda de cuota I10a | {len(i10a)} |

Cada resultado cita la sustitución y, si murió, la prueba que la detectó. `source-sha256.json` comprueba la restauración de las fuentes. Los supervivientes se clasifican según el nivel normativo del ADR en `docs/validation/i07c-mutations/`.

| Campaña medida | Programas | Comprobaciones | Nivel |
|---|---:|---:|---|
{campaign_table}

| Gemelos tipados/libres | Programas | Firmas reducidas | Excepciones adjudicadas |
|---|---:|---:|---:|
{typed_table}

| Gemelos de dominios pendientes | Programas | Comparables |
|---|---:|---:|
{domain_table}

| Medición complementaria | Valor |
|---|---:|
| Fuentes de proyección normativa | {normative['programs']} |
| Fuentes aceptadas cotejadas con `run` | {normative['accepted_programs']} |
| Mutantes con variaciones D/V adjudicadas | {normative['survivors_with_informative_differences']} |
| Programas con variaciones D/V adjudicadas | {normative['informative_changed_programs']} |
| Gemelos informativos | {twins['programs']} |
| Gemelos comparables | {twins['comparable_twins']} |
| Gemelos exceptuados por resolución posterior | {twins['deferred_resolution_exemptions']} |
| Diferencias permitidas por resolución posterior | {twins['deferred_resolution_differences']} |
| `deferred_release` sin la misma hoja | {deferred['missing_leaf_programs']} |
| `deferred_release` con diagnóstico adicional | {deferred['extra_diag_programs']} |

El arnés de gemelos comprueba campos de C con tipos completos y permutaciones; la campaña extensa de cuota reducida es informativa. La propiedad mínima de cuota reducida forma parte de la suite. Las puertas de fase siguen en `Pending.md`.
"""
    OUTPUT.write_text(content, encoding="utf-8", newline="\n")
    print(f"Estado regenerado: {len(entries)} mutaciones de producción")


if __name__ == "__main__":
    main()
