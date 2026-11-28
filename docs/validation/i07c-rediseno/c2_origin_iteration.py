#!/usr/bin/env python3
"""Short C2 origin loop over every v15f failure and fixed discriminators.

This is an iteration tool. It never replaces candidate_gate's complete corpus.
"""

import argparse
import hashlib
import json
import time
from pathlib import Path

import candidate_gate as gate
import c2_discriminator_gate as discriminators
from audit_families import source
from verify_corpora import FILE

HERE = Path(__file__).resolve().parent
FROZEN = HERE / "c2-ref-corpus-v15f.json"
BATTERIES = HERE / "r7-frozen-batteries.json"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory():
    baseline = json.loads(FROZEN.read_text(encoding="utf-8"))
    batteries = json.loads(BATTERIES.read_text(encoding="utf-8"))["corpora"]
    by_id = {entry["id"]: entry for entry in batteries}
    rows = {}
    for failure in baseline["failures"]:
        row_id, label = failure.rsplit("=", 1)
        if label not in ("A-unmodeled", "D"):
            continue
        corpus_id, index_text = row_id.rsplit(":", 1)
        entry = by_id[corpus_id]
        path = HERE / FILE[corpus_id.split("@")[0]]
        assert gate.sha(path) == entry["sha256"], corpus_id
        corpus = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(corpus, dict):
            corpus = corpus["progs"]
        index = int(index_text)
        program = source(corpus[index])
        rows[row_id] = {"source": program, "source_sha256": digest(program.encode()),
                        "profile": entry["profile"], "baseline_class": label,
                        "corpus_sha256": entry["sha256"], "line": index + 1,
                        "adr_classification": "PENDING_CLAUSE_REVIEW"}
    assert len(rows) == 120, len(rows)
    extras = {
        "literal_233": rows["b-projections-2026112401:233"],
        "scalar_996": {"source": source(json.loads((HERE / FILE["b-projections-2026112401"]).read_text(encoding="utf-8"))[996]),
                       "profile": "f0-0002-b-v2"},
        "match_72": {"source": json.loads((HERE / "r8-match-main.json").read_text(encoding="utf-8"))["rows"][72]["source"],
                     "profile": json.loads((HERE / "r8-match-main.json").read_text(encoding="utf-8"))["rows"][72]["profile"]},
        "r_ar_aar": {"source": "fn Main(){let ar=[1];let aar=[[1]];var r=[];let a=[r,ar,aar];r=[3];}",
                     "profile": "f0-0002-b-v2"},
        "w_array_eq": {"source": "fn Main(){var w=(if true then {return;} else {return;});let a0=[w,[1]==w];}",
                       "profile": "f0-0002-b-v2"},
    }
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1"):
        for name, (program, expected) in discriminators.CASES.items():
            if profile == "f0-0002-b-v2" and name in ("some_cycle", "rejected_shape_keeps_union_kind"):
                continue
            extras[f"discriminator:{profile}:{name}"] = {
                "source": program, "profile": profile, "expected_primary": expected,
                "expected_rel": discriminators.EXPECTED_REL.get(name, {}),
            }
    assert len([key for key in extras if key.startswith("discriminator:")]) == 38
    for row in extras.values():
        row.setdefault("source_sha256", digest(row["source"].encode()))
    return baseline, rows, extras


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path)
    parser.add_argument("--candidate-exe", type=Path)
    parser.add_argument("--case", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    baseline, rows, extras = inventory()
    report = {"schema": "i07c-c2-origin-iteration-v1", "baseline_sha256": gate.sha(FROZEN),
              "baseline_counts": {"A-unmodeled": 74, "D": 46},
              "rows": rows, "extras": extras, "complete": False}
    if args.main_exe or args.candidate_exe or args.case:
        if not (args.main_exe and args.candidate_exe and args.case):
            parser.error("both executables and --case are required together")
        started = time.monotonic()
        report["main_sha256"] = gate.sha(args.main_exe)
        report["candidate_sha256"] = gate.sha(args.candidate_exe)
        assert report["main_sha256"] == baseline["main_sha256"]
        results = {}
        for key, row in {**rows, **extras}.items():
            program, profile = row["source"], row["profile"]
            main_output = gate.execute(args.main_exe, program, profile, args.case)
            candidate_output = gate.execute(args.candidate_exe, program, profile, args.case)
            derived = gate.oracle.derive(program)
            results[key] = {"class": gate.classify(main_output, candidate_output, derived),
                            "main": gate.compact_match(main_output),
                            "candidate": gate.compact_match(candidate_output),
                            "oracle": derived,
                            "oracle_modeling": [name for name in ("error", "unmodeled", "no_fijado", "d_optional")
                                                if derived.get(name)]}
        report["results"] = results
        report["elapsed_seconds"] = round(time.monotonic() - started, 2)
        report["complete"] = True
        extra_failures = []
        for key, row in extras.items():
            observed = results[key]
            if key.startswith("discriminator:"):
                diagnostics = observed["candidate"]["diagnosticos"]
                primaries = [(item["code"], item["subcause"], *item["primary"])
                             for item in diagnostics]
                if primaries != [tuple(expected) for expected in row["expected_primary"]]:
                    extra_failures.append(key + ": primary/code")
                for index, span in row["expected_rel"].items():
                    actual = next((relation["span"] for relation in diagnostics[int(index)]["relations"]
                                   if relation["role"] == "expected_type"), None)
                    if actual != list(span):
                        extra_failures.append(key + f": expected_type {index}")
            elif key in ("match_72", "scalar_996", "r_ar_aar", "w_array_eq"):
                if observed["main"] != observed["candidate"]:
                    extra_failures.append(key + ": main differential")
        report["extra_failures"] = extra_failures
        report["failure_counts"] = {label: sum(value["class"] == label for key, value in results.items() if key in rows)
                                    for label in ("A", "A-unmodeled", "D", "H-divergent")}
        print(report["failure_counts"], "extras", len(extra_failures), f"{report['elapsed_seconds']}s")
        if report["elapsed_seconds"] >= 300:
            print("ITERATION BUDGET EXCEEDED: 300 s")
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
