#!/usr/bin/env python3
"""C2 causal discriminators from R6-final §7 and the C1 D/D' findings.

The expected diagnostics below are the source-site consequences of atomic
rejection, independent equalities, and constructor-specific E0103/E0113.
This supplements the frozen 20-corpus gate; it does not replace it.
"""

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


NEVER = "(if true then{return;}else{return;})"
RESOURCE = f"fn Main(){{var q={NEVER};q[0]=MockOpen(1);"
CASES = {
    "resource_without_later_shape": (RESOURCE + "}", []),
    "resource_rejected_array": (
        RESOURCE + "q=[];}", [("NX-E0102", "type_mismatch", 72, 74)]
    ),
    "resource_scalar_completion": (
        RESOURCE + "q=1;}", [("NX-E0102", "not_indexable", 53, 54)]
    ),
    "repeated_recursive_array": (
        "fn Main(){var q=[];q=[q];q=[q];}", [("NX-E0103", None, 16, 18)]
    ),
    "some_cycle": (
        f"fn Main(){{var q={NEVER};var r={NEVER};q=[r];r=Some(q);}}",
        [("NX-E0103", None, 98, 101), ("NX-E0113", None, 104, 111)],
    ),
    "ok_cycle": (
        f"fn Main(){{var q={NEVER};var r={NEVER};q=[r];r=Ok(q);}}",
        [("NX-E0103", None, 98, 101), ("NX-E0113", None, 104, 109)],
    ),
    "repeated_rejected_equality": (
        f"fn Main(){{var q={NEVER};q[0][0]=MockOpen(1);q=[q];q=[q];}}",
        [("NX-E0102", "type_mismatch", 76, 77),
         ("NX-E0102", "type_mismatch", 82, 83)],
    ),
    "rejected_shape_keeps_union_kind": (
        RESOURCE + "q=Some(q);q=[];}",
        [("NX-E0103", None, 72, 79), ("NX-E0102", "type_mismatch", 82, 84)],
    ),
    "deferred_operand_origin_before_assignment": (
        "fn Main(){var s=[];let a=[s[0],(if true then s else s)];s=[1];}",
        [("NX-E0102", "type_mismatch", 32, 53)],
    ),
    "failed_nested_if_no_retroactive_index": (
        "fn Main(){let x=1;var q=[];var r=[];let a0=[r[0],(if true then q[x][0] else (if r then [1] else q))];r=[1];}",
        [("NX-E0102", "type_mismatch", 80, 81)],
    ),
    "first_literal_operand_after_completion": (
        "fn Main(){var s=[];var t=[];let a0=[t[0],(if true then 1 else s[0]),s];t=[1];}",
        [("NX-E0102", "type_mismatch", 68, 69)],
    ),
    "first_operand_of_second_literal": (
        "fn Main(){var t=[];let a0=[t[0],[t]];let a1=[t[0],(if true then [1] else t)];t=[true];}",
        [("NX-E0102", "type_mismatch", 32, 35),
         ("NX-E0102", "type_mismatch", 51, 74),
         ("NX-E0102", "type_mismatch", 80, 84)],
    ),
    "first_literal_operand_with_index_variable": (
        "fn Main(){let x=1;var s=[];var t=[];let a0=[t[0],(if true then 1 else s[x]),s];t=[];}",
        [("NX-E0102", "type_mismatch", 76, 77)],
    ),
}

# The second literal in the middle case must select its own first operand.
EXPECTED_REL = {
    "first_literal_operand_after_completion": {0: (36, 40)},
    "first_operand_of_second_literal": {1: (45, 49)},
    "first_literal_operand_with_index_variable": {0: (44, 48)},
}


def observed(binary, case_path, source, profile, verb):
    case_path.write_bytes(source.encode("ascii"))
    result = subprocess.run(
        [str(binary), verb, str(case_path), "--profile", profile, "--format", "json"],
        capture_output=True, timeout=20, check=False,
    )
    if result.returncode not in (0, 1):
        raise RuntimeError(f"{verb} exit={result.returncode}: {result.stderr[:300]!r}")
    output = json.loads(result.stdout)
    if output.get("outcome", {}).get("kind") in ("driver_error", "internal_error"):
        raise RuntimeError(f"{verb} hard outcome={output['outcome']!r}")
    diagnostics = [
        (item["code"], item.get("subcause"), item["primary_span"]["start"],
         item["primary_span"]["end"])
        for item in output["diagnostics"]
    ]
    related = [
        next(((span["span"]["start"], span["span"]["end"])
              for span in item["related_spans"] if span["role"] == "expected_type"), None)
        for item in output["diagnostics"]
    ]
    return output["outcome"]["kind"], diagnostics, related


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"schema": "i07c-c2-discriminators-v1", "complete": True,
              "candidate_sha256": hashlib.sha256(args.candidate_exe.read_bytes()).hexdigest(),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "cases": {}, "failures": []}
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1"):
        for name, (source, expected) in CASES.items():
            # Some is introduced after the f0-0002 profile. The remaining
            # discriminators are valid in both profiles.
            if profile == "f0-0002-b-v2" and name in (
                "some_cycle", "rejected_shape_keeps_union_kind"
            ):
                continue
            key = f"{profile}:{name}"
            row = {"source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(),
                   "expected": expected, "observed": {}}
            report["cases"][key] = row
            for verb in ("check", "run"):
                try:
                    kind, got, related = observed(args.candidate_exe, args.case, source, profile, verb)
                    row["observed"][verb] = {"outcome": kind, "diagnostics": got,
                                             "expected_type_spans": related}
                    if got != expected or (kind == "rejected") != bool(expected):
                        report["failures"].append(f"{key}:{verb}: {kind}, {got!r}")
                    for index, span in EXPECTED_REL.get(name, {}).items():
                        if related[index] != span:
                            report["failures"].append(
                                f"{key}:{verb}: expected_type[{index}]={related[index]!r}, want {span!r}")
                except Exception as error:
                    report["failures"].append(f"{key}:{verb}: {error}")
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print(f"C2 discriminators {len(report['cases'])} profile-cases, "
          f"failures={len(report['failures'])}")
    for failure in report["failures"]:
        print(failure)
    if report["failures"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
