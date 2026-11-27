#!/usr/bin/env python3
"""Cross-check main's 15 D6 rows and freeze standalone ADR-if derivations."""

import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
INDEXES = (5, 7, 37, 57, 59, 62, 65, 67, 68, 69, 71, 72, 78, 79, 102)
EXTRA_CAUSE = {
    57: "match-arm join after q=r completes an arm",
    62: "match-arm join against the None-arm ar",
    69: "match-arm join after q=[3] completes the None arm",
    71: "match-arm join against the None-arm [r]",
    72: "match-arm join after the later r assignments",
    78: "normative if in the None arm, absent from the isolated Some-arm derivation",
    102: "later q=aar assignment after the match-arm join",
}
# Hand-transcribed consequences of the preserved D5 arm join and later
# assignments. These are expected diagnostics, not mutations of product code.
# Each tuple is (code, subcause, primary, expected, found, related expected,
# related found), with all spans in bytes of the complete match source.
EXTRA_EXPECTED = {
    57: ("NX-E1303", None, [260, 261], "Array<Array<Array<I64>>>", "Array<Array<Array<Array<I64>>>>", [161, 242], [260, 261]),
    62: ("NX-E1303", None, [232, 234], "Array<Array<I64>>", "Array<I64>", [152, 214], [232, 234]),
    69: ("NX-E1303", None, [223, 224], "I64", "Array<I64>", [143, 205], [223, 224]),
    71: ("NX-E1303", None, [231, 234], "Array<Array<I64>>", "Array<Array<Array<I64>>>", [152, 213], [231, 234]),
    72: ("NX-E1303", None, [302, 305], "Array<Array<I64>>", "Array<Array<Bool>>", [161, 284], [302, 305]),
    78: ("NX-E1303", None, [312, 313], "Array<Array<Array<I64>>>", "Array<I64>", [305, 306], [312, 313]),
    102: ("NX-E0102", "type_mismatch", [216, 219], "Array<I64>", "Array<Array<I64>>", [208, 209], [216, 219]),
}
spec = importlib.util.spec_from_file_location("i07c_rd_oracle", HERE.parents[2] / "scripts/i07c_rd_oracle.py")
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)


def main_signature(data):
    output = []
    for error in data.get("diagnostics", []):
        related = [{"role": item["role"], "span": [item["span"]["start"], item["span"]["end"]]}
                   for item in error.get("related_spans", [])]
        span = error["primary_span"]
        output.append({"code": error["code"], "subcause": error.get("subcause"),
                       "primary": [span["start"], span["end"]],
                       "expected_type": error.get("data", {}).get("expected_type"),
                       "found_type": error.get("data", {}).get("found_type"),
                       "relations": related})
    return {"outcome": data["outcome"]["kind"], "diagnosticos": output}


def standalone_arm(source):
    match = re.search(r"let a0=match move o\{Some\(n\)=>\{value: (.*?);\},None=>", source)
    assert match, source
    prefix = source[:source.index("let a0=match")].replace("let o:Option<I64>=Some(1);", "")
    suffix = source[source.index("}};", match.end()) + 3:]
    return prefix + "let a0=" + match.group(1) + ";" + suffix


def map_arm_diagnostics(original, standalone, diagnostics):
    match = re.search(r"let a0=match move o\{Some\(n\)=>\{value: (.*?);\},None=>", original)
    expression = match.group(1)
    old_expression = match.start(1)
    new_expression = standalone.index("let a0=") + len("let a0=")
    old_suffix = original.index("}};", match.end()) + 3
    new_suffix = new_expression + len(expression) + 1
    removed = len("let o:Option<I64>=Some(1);")
    def remap(n):
        if n < new_expression:
            return n + removed
        if n < new_suffix:
            return n + old_expression - new_expression
        return n + old_suffix - new_suffix
    result = []
    for diagnostic in diagnostics:
        mapped = dict(diagnostic)
        mapped["primary"] = [remap(n) for n in diagnostic["primary"]]
        for key in ("rel_expected", "rel_found"):
            if diagnostic.get(key) is not None:
                mapped[key] = [remap(n) for n in diagnostic[key]]
        result.append(mapped)
    return result


def derived_match_signature(diagnostic):
    relations = []
    for role, key in (("expected_type", "rel_expected"), ("found_type", "rel_found")):
        if diagnostic.get(key) is not None:
            relations.append({"role": role, "span": diagnostic[key]})
    return {"code": diagnostic["code"], "subcause": diagnostic.get("subcause"),
            "primary": diagnostic["primary"],
            "expected_type": diagnostic.get("expected_type") or None,
            "found_type": diagnostic.get("found_type") or None,
            "relations": relations}


def extra_signature(fields):
    code, subcause, primary, expected, found, rel_expected, rel_found = fields
    return {"code": code, "subcause": subcause, "primary": primary,
            "expected_type": expected, "found_type": found,
            "relations": [{"role": "expected_type", "span": rel_expected},
                          {"role": "found_type", "span": rel_found}]}


def canonical(value):
    if isinstance(value, dict):
        return {key: "<SOURCE>" if key == "file" and isinstance(item, str) else canonical(item)
                for key, item in value.items()}
    if isinstance(value, list):
        return [canonical(item) for item in value]
    return value


def main():
    executable, case_path = map(Path, sys.argv[1:3])
    source_rows = json.loads((HERE / "r8-match-main.json").read_text(encoding="utf-8"))["rows"]
    rows = []
    for index in INDEXES:
        item = source_rows[index]
        source = item["source"]
        assert hashlib.sha256(source.encode("ascii")).hexdigest() == item["source_sha256"]
        case_path.write_bytes(source.encode("ascii"))
        run = subprocess.run([str(executable), "check", str(case_path), "--profile", item["profile"], "--format", "json"],
                             capture_output=True, timeout=20, check=False)
        assert run.returncode in (0, 1), (index, run.returncode, run.stderr)
        observed = main_signature(json.loads(run.stdout))
        assert observed == item["expected_main"], (index, observed, item["expected_main"])
        arm_source = standalone_arm(source)
        derived = oracle.derive(arm_source)
        assert not derived.get("error") and not derived.get("unmodeled") and not derived.get("no_fijado"), (index, derived)
        mapped = map_arm_diagnostics(source, arm_source, derived["diagnostics"])
        expected_errors = [derived_match_signature(d) for d in mapped]
        if index in EXTRA_EXPECTED:
            expected_errors.append(extra_signature(EXTRA_EXPECTED[index]))
        expected_errors.sort(key=lambda d: d["primary"][0])
        proposed = {"outcome": "rejected", "diagnosticos": expected_errors}
        assert proposed == item["prototype_r6"], (index, proposed, item["prototype_r6"])
        actual_keys = {(d["code"], tuple(d["primary"])) for d in proposed["diagnosticos"]}
        derived_keys = {(d["code"], tuple(d["primary"])) for d in mapped}
        absent = [d for d in mapped if (d["code"], tuple(d["primary"])) not in actual_keys]
        additional = [d for d in proposed["diagnosticos"] if (d["code"], tuple(d["primary"])) not in derived_keys]
        assert all(d["code"] == "NX-E0103" for d in absent), (index, absent)
        assert all(derived_match_signature(d) in proposed["diagnosticos"] for d in mapped if d not in absent), index
        assert bool(additional) == (index in EXTRA_CAUSE), index
        rows.append({"index": index, "id": item["id"], "profile": item["profile"],
                     "source": source, "source_sha256": item["source_sha256"], "source_bytes": len(source.encode("ascii")),
                     "expected_main": observed,
                     "expected_main_full_json": canonical(json.loads(run.stdout)),
                     "standalone_if_source": arm_source,
                     "standalone_if_source_sha256": hashlib.sha256(arm_source.encode("ascii")).hexdigest(),
                     "standalone_if_adr": derived["diagnostics"],
                     "standalone_if_adr_spans_mapped_to_match": mapped,
                     "standalone_unresolved_constructors_completed_by_match": absent,
                     "additional_match_join_or_later_completion": additional,
                     "additional_cause": EXTRA_CAUSE.get(index),
                     "proposed_r6_full_match": proposed,
                     "clause": "F0-0002 4.1.2.1-5, 4.1.2.7, 13.9; D6 applies one if union algorithm inside and outside match; D5 preserves only arm joining and match state",
                     "validation_status": "A proposal; B must derive the full expected output for this row independently before C3"})
    document = {"schema": "i07c-r13-d6-v1", "base_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
                "main_binary_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
                "oracle_script_sha256": hashlib.sha256((HERE.parents[2] / "scripts/i07c_rd_oracle.py").read_bytes()).hexdigest(),
                "rule": "15 D6 rows: standalone if derivation is independent of match. Proposed full match output is separate and requires independent B validation per row; no PASS is asserted.",
                "rows": rows}
    (HERE / "r13-match-d6.json").write_text(json.dumps(document, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    def compact(output):
        errors = output["diagnosticos"]
        return ", ".join(f'{error["code"].removeprefix("NX-")} [{error["primary"][0]},{error["primary"][1]})'
                         for error in errors) or output["outcome"]
    table = ["# D6: 15 diferencias declaradas de match con if en brazo", "",
             "Cada fuente ASCII, su SHA-256, la salida JSON completa de main, la derivacion del if aislado y la salida normativa propuesta para R6 estan en `r13-match-d6.json`. Las 15 salidas de main se ejecutaron con el binario compilado de 31d91db. El generador coteja cada diagnostico del if aislado con la propuesta en codigo, subcausa, tipos, relaciones y spans tras desplazar los bytes. Los diagnosticos adicionales y su causa se enumeran por fila en el JSON. B debe derivar de forma independiente cada salida completa propuesta antes de C3. Los intervalos siguientes son bytes `[inicio,fin)`.", "",
             "| Fila | ID; bytes | main actual (codigo y primario) | R6 propuesto (codigo y primario) | Clausula |",
             "|---:|---|---|---|---|"]
    for row in rows:
        table.append(f'| {row["index"]} | `{row["id"]}`; {row["source_bytes"]} B | {compact(row["expected_main"])} | {compact(row["proposed_r6_full_match"])} | F0-0002 4.1.2.1-5, 13.9; D5/D6 |')
    (HERE / "r13-match-d6.md").write_text("\n".join(table) + "\n", encoding="utf-8")
    print("main verified", len(rows), "D6 rows")


if __name__ == "__main__":
    main()
