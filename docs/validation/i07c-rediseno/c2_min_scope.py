#!/usr/bin/env python3
"""Reproduce the D13 handoff rows on the minimum C2 binary."""

import argparse
import hashlib
import json
from pathlib import Path

import c2_discriminator_gate as discriminator

HERE = Path(__file__).resolve().parent
OWNED = ("RC1_read_context_variable", "ST1_equivalent",
         "RC1_read_context_variable:frozen")
CONTROLS = ("RC2_read_context_committed", "RC3_read_context_no_retroactive")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    directed = {row["id"]: row for row in json.loads(
        (HERE / "r8-directed74.json").read_text(encoding="utf-8"))["rows"]}
    frozen_rows = {row["id"]: row for row in json.loads(
        (HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["probes"]}
    report = {"schema": "i07c-c2-min-scope-v1", "complete": True,
              "candidate_sha256": sha(args.candidate_exe.read_bytes()),
              "discriminator_gate_sha256": sha((HERE / "c2_discriminator_gate.py").read_bytes()),
              "owned": {}, "controls": {}, "discriminators": {}, "failures": []}
    for name in OWNED + CONTROLS:
        frozen = name.endswith(":frozen")
        row = (frozen_rows if frozen else directed)[name.removesuffix(":frozen")]
        source = row["source"]
        if "[" not in source or ",(if " not in source:
            report["failures"].append(name + ": source lacks if literal operand")
        expected = [(code, start, end) for code, start, end in
                    row["expected_code_primary"]]
        evidence = {"source": source, "source_sha256": sha(source.encode("ascii")),
                    "profile": row["profile"], "expected_code_primary": expected,
                    "observed": {}}
        for verb in ("check", "run"):
            kind, observed, _ = discriminator.observed(args.candidate_exe, args.case,
                                                       source, row["profile"], verb)
            actual = [(code, start, end) for code, _, start, end in observed]
            evidence["observed"][verb] = {"kind": kind, "diagnostics": observed,
                                          "matches_oracle": actual == expected}
            should_match = name in CONTROLS
            if (actual == expected) != should_match:
                report["failures"].append(name + ": unexpected handoff status")
        (report["controls"] if name in CONTROLS else report["owned"])[name] = evidence
        if frozen:
            direct = directed["RC1_read_context_variable"]
            assert (source, row["profile"], row["expected_code_primary"]) == (
                direct["source"], direct["profile"], direct["expected_code_primary"])
    for name in sorted(discriminator.C3_OPERAND_IF):
        source, expected = discriminator.CASES[name]
        if ",(if " not in source:
            report["failures"].append(name + ": source lacks if literal operand")
        for profile in ("f0-0002-b-v2", "f0-0003-b-v1"):
            evidence = {"source": source, "source_sha256": sha(source.encode("ascii")),
                        "expected": expected, "profile": profile, "observed": {}}
            for verb in ("check", "run"):
                kind, observed, related = discriminator.observed(args.candidate_exe, args.case,
                                                                 source, profile, verb)
                evidence["observed"][verb] = {"kind": kind, "diagnostics": observed,
                                              "expected_type_spans": related}
                if observed == expected:
                    report["failures"].append(f"{name}:{profile}:{verb}: unexpectedly passes C3")
            report["discriminators"][f"{profile}:{name}"] = evidence
    if len(report["owned"]) != 3 or len(report["discriminators"]) != 4:
        report["failures"].append("wrong handoff cardinality")
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("D13 scope", len(report["owned"]), "owned rows,",
          len(report["discriminators"]) * 2, "discriminator invocations; audit failures",
          len(report["failures"]))
    if report["failures"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
