#!/usr/bin/env python3
"""Reproduce failures outside the frozen C2 members with source and JSONL.

This is an investigation aid; it does not discharge any cross-gate failure.
"""

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path

import candidate_gate as gate
from c2_family_check import classify as causal_classify

HERE = Path(__file__).resolve().parent


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", type=Path, required=True)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--trace-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    cross = json.loads(args.gate.read_text(encoding="utf-8"))
    if not cross["complete"] or len(cross["class_by_row"]) != 151188:
        raise ValueError("complete 151188-row cross gate required")
    if cross["main_sha256"] != gate.sha(args.main_exe) or cross["candidate_sha256"] != gate.sha(args.candidate_exe):
        raise ValueError("gate binaries differ")
    frozen = json.loads((HERE / "c2-family-members-v15f.json").read_text(encoding="utf-8"))
    members = {row for family in frozen["families"].values() for row in family["members"]}
    extras = []
    non_rows = []
    for failure in cross["failures"]:
        row_id, _, label = failure.partition("=")
        if row_id not in cross["class_by_row"]:
            non_rows.append(failure)
        elif label not in ("A", "A-unmodeled", "D", "H-divergent"):
            non_rows.append(failure)
        elif row_id not in members:
            extras.append((row_id, label))
    definitions = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    entries = {entry["id"]: entry for entry in definitions}
    observations = {}
    for row_id, label in sorted(set(extras)):
        corpus_id, index_text = row_id.rsplit(":", 1)
        entry = entries[corpus_id]
        corpus = HERE / gate.FILE[corpus_id.split("@")[0]]
        if gate.sha(corpus) != entry["sha256"]:
            raise ValueError(f"changed corpus: {corpus_id}")
        rows = json.loads(corpus.read_text(encoding="utf-8"))
        if isinstance(rows, dict):
            rows = rows["progs"]
        source = gate.source(rows[int(index_text)])
        profile = entry["profile"]
        main_output = gate.execute(args.main_exe, source, profile, args.case)
        candidate_output = gate.execute(args.candidate_exe, source, profile, args.case)
        actual_label = gate.classify(main_output, candidate_output, gate.oracle.derive(source))
        if actual_label != label:
            raise ValueError(f"{row_id}: gate class {label}, reproduced {actual_label}")
        args.case.write_bytes(source.encode("utf-8"))
        env = os.environ.copy()
        env["NEXO_ORIGIN_TRACE"] = "1"
        run = subprocess.run([str(args.trace_exe), "check", str(args.case), "--profile", profile,
                              "--format", "json"], env=env, capture_output=True, timeout=30, check=False)
        if run.returncode not in (0, 1):
            raise RuntimeError(f"{row_id}: trace exit {run.returncode}")
        if gate.compact_match(json.loads(run.stdout)) != gate.compact_match(candidate_output):
            raise ValueError(f"{row_id}: trace build changed output")
        lines = run.stderr.decode("utf-8", errors="replace").splitlines()
        if any(not line.startswith("C2_ORIGIN\t") for line in lines):
            raise ValueError(f"{row_id}: unrelated stderr")
        events = [json.loads(line.removeprefix("C2_ORIGIN\t")) for line in lines]
        family, witness = causal_classify(gate.compact_match(main_output),
                                          gate.compact_match(candidate_output), events)
        observations[row_id] = {"class": label, "profile": profile, "source": source,
                                "source_sha256": sha(source.encode("ascii")),
                                "main": gate.compact_match(main_output),
                                "candidate": gate.compact_match(candidate_output),
                                "family_probe": family, "witness": witness,
                                "events": events}
        print(row_id, label, family, flush=True)
    report = {"schema": "i07c-c2-cross-extra-probe-v1", "cross_sha256": gate.sha(args.gate),
              "main_sha256": gate.sha(args.main_exe), "candidate_sha256": gate.sha(args.candidate_exe),
              "trace_sha256": gate.sha(args.trace_exe), "non_row_failures": non_rows,
              "rows": observations}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(len(observations), "extra rows;", len(non_rows), "non-row failures")


if __name__ == "__main__":
    main()
