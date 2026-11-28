#!/usr/bin/env python3
"""Freeze B r25 re-adjudication without treating the partial oracle as authority."""

import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ITERATION = HERE / "c2-d8-iteration-pb.json"
OLD = HERE / "c2-d8-blockers-a46.json"
TRACE = HERE / "c2-r25-trace-120.json.gz"
OUT = HERE / "c2-r25-ledger.json"
SAMPLE_MD = HERE / "c2-r25-sample.md"

SHARED_SHAPE = {
    "b-projections-2026112401:2919",
    "b-projections-t5-2026112402:468",
    "b-projections-2026112403:2532",
    "c1:11474", "c1:18686", "b:2709", "c1-nueva:1530",
}
FALSE_PARTIAL = {"b-projections-t5-2026112402:761",
                 "b-projections-t5-2026112402:2072"}

B_DECLARABLE = [
    "b-projections-2026112401:280", "b-projections-2026112401:550",
    "b-projections-2026112401:1183", "b-projections-t5-2026112402:1266",
    "c2:2858", "c2:8742", "c2:15473",
    "c2@f0-0003-b-v1:7609", "s7021993:3267",
]
ADDITIONAL = [
    "b-projections-2026112401:1196", "b-projections-2026112401:2977",
    "b-projections-t5-2026112402:820", "b-projections-t5-2026112402:838",
    "c1-nueva:631", "b-projections-2026112403:1916",
    "c1:921", "c1:11189", "c1:13247", "c1:15115", "c1:15304",
    "c1:21363", "c2:176", "c2@f0-0003-b-v1:176", "b:3042", "b:3122",
    "s7021993:278", "s7021993:2391",
]
SAMPLE = B_DECLARABLE + ["b-projections-2026112401:1390", "c2:9591",
                         "c2@f0-0003-b-v1:9591"] + ADDITIONAL


def signature(result):
    return [[d["code"], d["subcause"], d["primary"]]
            for d in result["diagnosticos"]]


def main():
    data = json.loads(ITERATION.read_text(encoding="utf-8"))
    old = json.loads(OLD.read_text(encoding="utf-8"))
    traces = json.loads(gzip.decompress(TRACE.read_bytes()))["rows"]
    partial = {}
    for row_id, old_row in old["rows"].items():
        if not old_row["missing_required_c"]:
            continue
        if row_id in SHARED_SHAPE:
            status = "SHARED_WITH_MAIN"
            basis = "B r25 and F0-0002 §4.1.2.2: rejected scalar cannot fix Array element"
        elif row_id in FALSE_PARTIAL:
            status = "PARTIAL_ORACLE_FALSE_POSITIVE"
            basis = ("§4.1.2.1-2,7: earlier r[0] fixes prefix" if row_id.endswith(":761")
                     else "§4.1.2.5: q condition failure taints dependent r=q")
        else:
            status = "INCONCLUSIVE_MANUAL_DERIVATION_PENDING"
            basis = "partial oracle alone cannot establish independent C"
        observed = data["results"][row_id]
        partial[row_id] = {
            "source_sha256": data["rows"][row_id]["source_sha256"],
            "status": status, "basis": basis,
            "partial_oracle_claims": old_row["missing_required_c"],
            "main": signature(observed["main"]),
            "candidate": signature(observed["candidate"]),
        }
    assert len(partial) == 16 and len(SHARED_SHAPE) == 7
    assert len(SAMPLE) == len(set(SAMPLE)) == 30
    sample = {}
    for row_id in SAMPLE:
        row = data["rows"][row_id]
        result = data["results"][row_id]
        if row_id in B_DECLARABLE:
            status = "CANDIDATE_ADR_CONFORMING_IN_B_R25_SAMPLE"
            basis = "B r25: specific relation or one diagnosis per union"
        elif row_id.endswith(":1390"):
            status = "CANDIDATE_ROUTE_DEFECT"
            basis = "§4.1.2.4 and §13.15: aar witness is ar [123,125), not q [81,82)"
        elif row_id == "b-projections-t5-2026112402:820":
            status = "CANDIDATE_MISSING_CONSTRUCTOR_C"
            basis = "§4.1.2.2: s=[] [70,72) remains unresolved; both uses of s[0] lack a compatible concrete element witness"
        elif row_id.endswith(":9591"):
            status = "ADR_AMBIGUOUS_KEEP_MAIN_D9"
            basis = "nested if through value block; keep main's C at [166,170)"
        else:
            missing = [diag for diag in signature(result["main"])
                       if diag not in signature(result["candidate"])]
            if len(missing) != 1 or missing[0][0] != "NX-E1303":
                raise ValueError(f"{row_id}: expected one omitted join diagnostic: {missing}")
            site = missing[0][2]
            contexts = [event for event in traces[row_id]["trace"]
                        if event["event"] == "if_join_context"
                        and event["if_span"][0] <= site[0]
                        and site[1] <= event["if_span"][1]]
            context = min(contexts, key=lambda event: event["if_span"][1] - event["if_span"][0])
            earlier = [diag for diag in signature(result["candidate"])
                       if diag[0] == "NX-E1303" and context["if_span"][0] <= diag[2][0]
                       and diag[2][1] <= context["if_span"][1] and diag[2][0] < site[0]]
            if context["external_context"] or not earlier:
                raise ValueError(f"{row_id}: join limit predicate not established")
            status = "CANDIDATE_ADR_CONFORMING_ONE_PER_UNION"
            basis = ("§4.1.2 N(d): second alternative of same if without external concrete "
                     f"context; first C at {earlier[0][2]}; owner AST {context['owner_ordinal']}")
        decisions = [event for event in traces[row_id]["trace"] if event["event"] == "finish"]
        assert decisions and all(event["source_ordinal"] is not None for event in decisions)
        sample[row_id] = {
            "source_sha256": row["source_sha256"], "profile": row["profile"],
            "status": status, "basis": basis,
            "main": signature(result["main"]), "candidate": signature(result["candidate"]),
            "trace_decisions": len(decisions),
            "trace_rejected": sum(event["state"] == "rejected" for event in decisions),
        }
    report = {
        "schema": "c2-r25-adjudication-v1",
        "iteration_sha256": hashlib.sha256(ITERATION.read_bytes()).hexdigest(),
        "trace_sha256": hashlib.sha256(TRACE.read_bytes()).hexdigest(),
        "partial_c_audit": partial,
        "sample_30_of_original_102": sample,
        "sample_counts": {
            "executed_and_traced": 30,
            "adjudicated_candidate_conforming": 26,
            "adjudicated_candidate_defect": 2,
            "adr_ambiguous_keep_main": 2,
            "inconclusive": 0,
        },
        "estimate": "Directed sample: 2/30 intrinsic candidate defects, plus 2/30 D9 main-preservation changes = 4/30 (13.3%) requiring correction. The sample is selected by known families, so this is a within-sample rate, not an unbiased estimate for all 102. B r25's 10-25% remains a working range.",
    }
    OUT.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    lines = [
        "# I07c C2 — muestra dirigida de 30/102, B r25 + D9",
        "",
        "Cada fila fue ejecutada en `main` y candidato con el perfil congelado y tiene ",
        "decisiones JSONL de igualdad. SHA de fuente, firmas completas y traza están en ",
        "`c2-r25-ledger.json` y `c2-r25-trace-120.json.gz`. Los nueve dictámenes ",
        "iniciales de conformidad proceden de B r25. Los otros 17 conformes usan ",
        "el límite normativo de una denuncia por unión sin contexto externo, ",
        "comprobado contra el `owner_ordinal` y la salida; el oráculo parcial ",
        "no decide el veredicto.",
        "",
        "| Fila | Estado | `main` / candidato (códigos y primarios) | Decisiones rechazadas en traza |",
        "|---|---|---|---:|",
    ]
    for row_id, item in sample.items():
        brief = {
            "CANDIDATE_ADR_CONFORMING_IN_B_R25_SAMPLE": "PASA según B r25",
            "CANDIDATE_ADR_CONFORMING_ONE_PER_UNION": "PASA: una por unión",
            "CANDIDATE_ROUTE_DEFECT": "FALLA: ruta de testigo",
            "CANDIDATE_MISSING_CONSTRUCTOR_C": "FALLA: falta E0103",
            "ADR_AMBIGUOUS_KEEP_MAIN_D9": "D9: conservar main",
        }[item["status"]]
        def concise(rows):
            return ", ".join(f"{code}@{primary[0]}" for code, _, primary in rows)
        lines.append(f"| `{row_id}` | {brief} | {concise(item['main'])} / {concise(item['candidate'])} | {item['trace_rejected']} |")
    lines += [
        "", "26 conformes, 2 defectos propios del candidato (1390 y 820), 2 filas de ",
        "laguna con conservación D9 (9591 ×2). Son **4/30 = 13,3 %** que exigen ",
        "corrección en esta muestra dirigida; 2/30 = 6,7 % son defectos intrínsecos ",
        "ya derivados. La selección deliberada por familias no permite extrapolar ",
        "esas tasas a las 102. El rango 10–25 % de B r25 sigue como hipótesis de ",
        "trabajo. Restan 72 filas fuera de la muestra.", "",
    ]
    SAMPLE_MD.write_text("\n".join(lines), encoding="utf-8")
    print(f"{len(partial)} partial-C rows, {len(sample)} sampled rows; {OUT}")


if __name__ == "__main__":
    main()
