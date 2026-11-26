#!/usr/bin/env python3
"""Frozen A/B/C/D cross gate for the R6 candidate against main and ADR.

A: main == ADR, candidate != ADR (regression).
B: main != ADR, candidate == ADR (repair).
C: main != ADR, candidate == main (still historical).
D: main != ADR, candidate differs from both (unadjudicated).
H: historical hard failure in main, conserved through C5 and fixed in C6.
The full corpus is required for a gate. --limit is a report-only smoke run.
"""

import argparse
import collections
import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
import i07c_rd_oracle as oracle  # noqa: E402
from verify_corpora import FILE  # noqa: E402
from audit_families import project_main, project_oracle, source  # noqa: E402


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(binary, source_text, profile, case_path):
    case_path.write_bytes(source_text.encode("utf-8"))
    run = subprocess.run([str(binary), "check", str(case_path), "--profile", profile, "--format", "json"],
                         capture_output=True, timeout=20, check=False)
    if run.returncode not in (0, 1, 2):
        raise RuntimeError(f"CLI exit {run.returncode}: {run.stderr[:300]!r}")
    data = json.loads(run.stdout)
    if run.returncode == 2 and data.get("outcome") == {"kind": "driver_error", "reason": "internal_error"}:
        return data
    if run.returncode == 2:
        raise RuntimeError(f"unexpected CLI exit 2: {data.get('outcome')!r}")
    if data.get("outcome", {}).get("kind") not in ("checked", "rejected"):
        raise RuntimeError(f"non-normative check outcome: {data.get('outcome')!r}")
    return data


def signature(data):
    return [data["outcome"]["kind"], project_main(data)]


def oracle_signature(derived):
    return ["rejected" if derived["diagnostics"] else "checked", project_oracle(derived)]


def classify(main_output, candidate_output, derived):
    if main_output["outcome"] == {"kind": "driver_error", "reason": "internal_error"}:
        return "H" if candidate_output == main_output else "H-divergent"
    if derived.get("error") or derived.get("unmodeled") or derived.get("no_fijado") or derived.get("d_optional"):
        return "U" if candidate_output == main_output else "A-unmodeled"
    m, c, o = signature(main_output), signature(candidate_output), oracle_signature(derived)
    if m == o:
        return "OK" if c == o else "A"
    if c == o:
        return "B"
    if c == m:
        return "C"
    return "D"


def compact_match(data):
    errors = []
    for item in data.get("diagnostics", []):
        rels = [{"role": r["role"], "span": [r["span"]["start"], r["span"]["end"]]}
                for r in item.get("related_spans", [])]
        p = item["primary_span"]
        errors.append({"code": item["code"], "subcause": item.get("subcause"),
                       "primary": [p["start"], p["end"]],
                       "expected_type": item.get("data", {}).get("expected_type"),
                       "found_type": item.get("data", {}).get("found_type"), "relations": rels})
    return {"outcome": data["outcome"]["kind"], "diagnosticos": errors}


def canonical(value):
    if isinstance(value, dict):
        return {key: "<SOURCE>" if key == "file" and isinstance(item, str) else canonical(item)
                for key, item in value.items()}
    if isinstance(value, list):
        return [canonical(item) for item in value]
    return value


def match_gate(main_bin, candidate_bin, case, commit):
    matrix = json.loads((HERE / "r8-match-main.json").read_text(encoding="utf-8"))["rows"]
    d6 = {r["index"]: r for r in json.loads((HERE / "r13-match-d6.json").read_text(encoding="utf-8"))["rows"]}
    exterior = json.loads((HERE / "r13-match-exterior.json").read_text(encoding="utf-8"))["rows"]
    assert len(matrix) == 103 and len(d6) == 15 and len(exterior) >= 6
    failures = []
    for i, row in enumerate(matrix):
        src = row["source"]
        m = execute(main_bin, src, row["profile"], case)
        c = execute(candidate_bin, src, row["profile"], case)
        if compact_match(m) != row["expected_main"]:
            failures.append(f"main fixture changed: match {i}")
        if i in d6 and canonical(m) != d6[i]["expected_main_full_json"]:
            failures.append(f"main full JSON changed: D6 match {i}")
        if commit in ("C0", "C1", "C2") or i not in d6:
            if c != m:
                failures.append(f"main mismatch: match {i}")
        elif compact_match(c) != d6[i]["proposed_r6_full_match"]:
            failures.append(f"D6 mismatch: match {i}")
    if commit in ("C3", "C4", "C5", "C6"):
        for row in exterior:
            m = execute(main_bin, row["source"], row["profile"], case)
            c = execute(candidate_bin, row["source"], row["profile"], case)
            if canonical(m) != row["expected_main"] or c != m:
                failures.append(f"exterior mismatch: {row['id']}")
    return failures


def f2_ne_gate(main_bin, candidate_bin, case, commit):
    if commit == "C1":
        return []
    row = json.loads((HERE / "r13-f2-ne.json").read_text(encoding="utf-8"))["rows"][0]
    m = execute(main_bin, row["source"], row["profile"], case)
    c = execute(candidate_bin, row["source"], row["profile"], case)
    failures = []
    if canonical(m) != row["expected_main"]:
        failures.append("F2 NE main fixture changed")
    if signature(c) != oracle_signature(row["expected_adr"]):
        failures.append("F2 NE candidate != ADR")
    return failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True, help="stable source path shared by both executables")
    parser.add_argument("--commit", choices=[f"C{i}" for i in range(7)], required=True)
    parser.add_argument("--previous", type=Path, help="previous commit's complete gate JSON")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--limit", type=int, help="report-only smoke limit per corpus; never proves a gate")
    args = parser.parse_args()
    if args.commit == "C0":
        raise SystemExit("C0 keeps its existing exact-main gate; this cross gate starts at C1")
    definitions = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    prior = json.loads(args.previous.read_text(encoding="utf-8")) if args.previous else None
    if prior is not None:
        assert prior["complete"] and prior["commit"] == f"C{int(args.commit[1:]) - 1}"
        assert prior["main_sha256"] == sha(args.main_exe)
        assert prior["oracle_sha256"] == sha(HERE.parents[2] / "scripts/i07c_rd_oracle.py")
    report = {"schema": "i07c-r13-candidate-gate-v1", "commit": args.commit,
              "complete": args.limit is None, "main_sha256": sha(args.main_exe),
              "candidate_sha256": sha(args.candidate_exe),
              "oracle_sha256": sha(HERE.parents[2] / "scripts/i07c_rd_oracle.py"),
              "classifier_sha256": sha(Path(__file__)), "corpora": {}, "class_by_row": {},
              "failures": []}
    cache = {}
    hard_rows = json.loads((HERE / "r14-main-hard.json").read_text(encoding="utf-8"))["rows"]
    for entry in definitions:
        ident = entry["id"]
        corpus = HERE / FILE[ident.split("@")[0]]
        assert sha(corpus) == entry["sha256"], ident
        rows = json.loads(corpus.read_text(encoding="utf-8"))
        if isinstance(rows, dict):
            rows = rows["progs"]
        assert len(rows) == entry["rows"], ident
        counts = collections.Counter()
        for index, item in enumerate(rows[:args.limit]):
            src = source(item)
            key = (entry["profile"], src)
            if key not in cache:
                m = execute(args.main_exe, src, entry["profile"], args.case)
                c = m if args.main_exe.resolve() == args.candidate_exe.resolve() else execute(args.candidate_exe, src, entry["profile"], args.case)
                cache[key] = classify(m, c, oracle.derive(src))
            label = cache[key]
            row_id = f"{ident}:{index}"
            if row_id in hard_rows:
                expected = hard_rows[row_id]
                assert sha(corpus) == expected["corpus_sha256"]
                assert hashlib.sha256(src.encode("ascii")).hexdigest() == expected["source_sha256"]
                m = execute(args.main_exe, src, entry["profile"], args.case)
                c = execute(args.candidate_exe, src, entry["profile"], args.case)
                if m["outcome"] != {"kind": "driver_error", "reason": "internal_error"}:
                    report["failures"].append(row_id + "=main-hard-fixture-changed")
                if args.commit == "C6":
                    label = "B" if signature(c) == expected["expected_adr_signature"] else "H-divergent"
                else:
                    label = "H" if c == m else "H-divergent"
            elif label.startswith("H"):
                report["failures"].append(row_id + "=unexpected-main-hard")
            report["class_by_row"][row_id] = label
            counts[label] += 1
            if label in ("A", "A-unmodeled", "D", "H-divergent"):
                report["failures"].append(row_id + "=" + label)
            if prior is not None and prior["class_by_row"].get(row_id) == "B" and label == "C":
                report["failures"].append(row_id + "=oracle-to-main")
        report["corpora"][ident] = {"rows": min(len(rows), args.limit) if args.limit is not None else len(rows),
                                     "profile": entry["profile"], "sha256": entry["sha256"],
                                     "counts": dict(sorted(counts.items()))}
        print(ident, report["corpora"][ident]["counts"], flush=True)
    if args.limit is None:
        report["failures"].extend(match_gate(args.main_exe, args.candidate_exe, args.case, args.commit))
        report["failures"].extend(f2_ne_gate(args.main_exe, args.candidate_exe, args.case, args.commit))
        if args.commit == "C6":
            report["failures"].extend(row_id + "=still-main" for row_id, label in report["class_by_row"].items() if label == "C")
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    if args.limit is not None:
        print("INCOMPLETE SMOKE: never evidence of gate success")
    elif report["failures"]:
        print("GATE FAIL", len(report["failures"]), file=sys.stderr)
        raise SystemExit(1)
    else:
        print("GATE GREEN", sum(v["rows"] for v in report["corpora"].values()))


if __name__ == "__main__":
    main()
