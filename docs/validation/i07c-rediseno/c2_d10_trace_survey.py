#!/usr/bin/env python3
"""Inspect published facts for relation-only D10 repros without classifying them."""

import argparse
import json
import os
import subprocess
from pathlib import Path

import candidate_gate as gate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--trace-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    audit = json.loads(args.audit.read_text(encoding="utf-8"))
    env = os.environ.copy()
    env["NEXO_ORIGIN_TRACE"] = "1"
    rows = []
    for row in audit["rows"]:
        if not row["relation_only"]:
            continue
        args.case.write_bytes(row["source"].encode("ascii"))
        run = subprocess.run([str(args.trace_exe), "check", str(args.case),
                              "--profile", "f0-0003-b-v1", "--format", "json"],
                             capture_output=True, env=env, timeout=30, check=False)
        if run.returncode not in (0, 1):
            raise RuntimeError(f"trace exit {run.returncode} on seed {row['seed']}")
        output = json.loads(run.stdout)
        if gate.signature(output) != row["candidate"]:
            raise ValueError(f"trace output differs for seed {row['seed']}")
        lines = run.stderr.decode("utf-8").splitlines()
        if any(not line.startswith("C2_ORIGIN\t") for line in lines):
            raise ValueError(f"unexpected stderr on seed {row['seed']}")
        events = [json.loads(line.split("\t", 1)[1]) for line in lines]
        changed = []
        for left, right in zip(row["main"][1], row["candidate"][1]):
            if left == right:
                continue
            old, new = left[5], right[5]
            selects = [event for event in events if event.get("event") == "select"
                       and event.get("chosen") == new]
            facts = [fact for event in events if event.get("event") == "enter"
                     for component in event.get("components", [])
                     for fact in component.get("facts", [])
                     if fact.get("concrete") == old or fact.get("operand") == old
                     or fact.get("source") == old]
            changed.append({"primary": right[2], "main_relation": old,
                            "candidate_relation": new, "selects": selects,
                            "main_site_facts": facts})
        rows.append({"seed": row["seed"], "source_sha256": row["source_sha256"],
                     "changed": changed})
    report = {"schema": "i07c-c2-d10-trace-survey-v1", "audit_sha256": gate.sha(args.audit),
              "trace_exe_sha256": gate.sha(args.trace_exe), "rows": rows}
    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print("relation-only traces", len(rows))


if __name__ == "__main__":
    main()
