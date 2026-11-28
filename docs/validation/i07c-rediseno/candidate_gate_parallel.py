#!/usr/bin/env python3
"""Execute the frozen candidate_gate classifier on every corpus row with four workers.

Each worker uses a private case path. `--limit` is only for equivalence checks;
it marks the report incomplete and cannot support a gate verdict.
"""

import argparse
import collections
import concurrent.futures
import functools
import hashlib
import json
import threading
from pathlib import Path

import candidate_gate as gate
from audit_families import source
from verify_corpora import FILE

HERE = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--commit", choices=("C1", "C2"), required=True)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not 1 <= args.workers <= 4:
        parser.error("1–4 workers required")
    prior = json.loads(args.previous.read_text(encoding="utf-8")) if args.previous else None
    if args.commit == "C2":
        if (prior is None or not prior["complete"] or prior["commit"] != "C1" or
                prior["main_sha256"] != gate.sha(args.main_exe) or
                prior["oracle_sha256"] != gate.sha(Path(gate.oracle.__file__))):
            parser.error("C2 needs a complete C1 report with the same main and oracle")
    elif prior is not None:
        parser.error("C1 has no previous gate")
    definitions = json.loads((HERE / "r7-frozen-batteries.json").read_text(
        encoding="utf-8"))["corpora"]
    hard_rows = json.loads((HERE / "r14-main-hard.json").read_text(
        encoding="utf-8"))["rows"]
    report = {"schema": "i07c-r13-candidate-gate-parallel-v1", "commit": args.commit,
              "complete": args.limit is None,
              "main_sha256": gate.sha(args.main_exe),
              "candidate_sha256": gate.sha(args.candidate_exe),
              "oracle_sha256": gate.sha(Path(gate.oracle.__file__)),
              "classifier_sha256": gate.sha(HERE / "candidate_gate.py"),
              "runner_sha256": gate.sha(Path(__file__)), "workers": args.workers,
              "corpora": {}, "class_by_row": {}, "failures": []}
    tasks = []
    for entry in definitions:
        corpus = HERE / FILE[entry["id"].split("@")[0]]
        assert gate.sha(corpus) == entry["sha256"], entry["id"]
        data = json.loads(corpus.read_text(encoding="utf-8"))
        rows = data["progs"] if isinstance(data, dict) else data
        assert len(rows) == entry["rows"], entry["id"]
        for index, item in enumerate(rows[:args.limit]):
            tasks.append((entry, index, source(item)))

    @functools.lru_cache(maxsize=None)
    def classify(profile, src):
        case = args.case.with_name(f"{args.case.stem}-{threading.get_ident()}.n")
        m = gate.execute(args.main_exe, src, profile, case)
        c = gate.execute(args.candidate_exe, src, profile, case)
        return gate.classify(m, c, gate.oracle.derive(src))

    def evaluate(task):
        entry, index, src = task
        row_id = f"{entry['id']}:{index}"
        label = classify(entry["profile"], src)
        issues = []
        if row_id in hard_rows:
            expected = hard_rows[row_id]
            assert hashlib.sha256(src.encode("ascii")).hexdigest() == expected["source_sha256"]
            case = args.case.with_name(f"{args.case.stem}-{threading.get_ident()}.n")
            m = gate.execute(args.main_exe, src, entry["profile"], case)
            c = gate.execute(args.candidate_exe, src, entry["profile"], case)
            if m["outcome"] != {"kind": "driver_error", "reason": "internal_error"}:
                issues.append(row_id + "=main-hard-fixture-changed")
            label = "H" if c == m else "H-divergent"
        elif label.startswith("H"):
            issues.append(row_id + "=unexpected-main-hard")
        if label in ("A", "A-unmodeled", "D", "H-divergent"):
            issues.append(row_id + "=" + label)
        if prior is not None and prior["class_by_row"].get(row_id) == "B" and label == "C":
            issues.append(row_id + "=oracle-to-main")
        return entry, row_id, label, issues

    counts = collections.Counter()
    current_id = None
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for entry, row_id, label, issues in pool.map(evaluate, tasks):
            if current_id is not None and entry["id"] != current_id:
                previous = report["corpora"][current_id]
                previous["counts"] = dict(sorted(counts.items()))
                print(current_id, previous["counts"], flush=True)
                counts.clear()
            current_id = entry["id"]
            if current_id not in report["corpora"]:
                report["corpora"][current_id] = {
                    "rows": min(entry["rows"], args.limit) if args.limit is not None
                    else entry["rows"], "profile": entry["profile"],
                    "sha256": entry["sha256"]}
            report["class_by_row"][row_id] = label
            counts[label] += 1
            report["failures"].extend(issues)
    if current_id is not None:
        report["corpora"][current_id]["counts"] = dict(sorted(counts.items()))
        print(current_id, report["corpora"][current_id]["counts"], flush=True)
    if args.limit is None:
        report["failures"].extend(gate.match_gate(args.main_exe, args.candidate_exe,
                                                   args.case, args.commit))
        report["failures"].extend(gate.f2_ne_gate(args.main_exe, args.candidate_exe,
                                                   args.case, args.commit))
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("parallel gate", len(report["class_by_row"]), "rows, failures",
          len(report["failures"]), flush=True)
    if report["failures"] and args.limit is None:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
