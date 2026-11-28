#!/usr/bin/env python3
"""C2/C3 causal discriminators from R6-final and the C1 D/D' findings.

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
    "literal_origin_after_prior_if": (
        "fn Main(){let x=1;var s=[];var u=[];let a1=[(if false then s[0] else u)];let a2=[s[x],(if false then u else u),1];u=[true];}",
        [("NX-E0102", "type_mismatch", 111, 112)],
    ),
    "literal_origin_after_nested_array": (
        "fn Main(){let x=1;var s=[];var t=[];var u=[];let a0=[s[x],[u]];let a1=[s[0],(if false then [s] else t)];u=[true];}",
        [("NX-E0102", "type_mismatch", 92, 93)],
    ),
    "literal_origin_after_if_projection": (
        "fn Main(){let x=1;var s=[];var t=[];var u=[];let a0=[(if false then s[x] else u)];let a1=[s[0],(if true then t[x] else u[1])];u=[1];}",
        [("NX-E0102", "type_mismatch", 119, 123)],
    ),
    "literal_origin_after_two_literals": (
        "fn Main(){let x=1;var s=[];var t=[];var u=[];let a0=[t[x],[u]];let a1=[t[1],s[0]];let a2=[s[0],(if true then s else t[0])];u=[true];}",
        [("NX-E0102", "type_mismatch", 109, 110)],
    ),
    "later_union_branch_preserves_deferred_equality": (
        "fn Main(){let x=1;var s=[];var t=[];var u=[];let a0=[u[0],t[1]];let a1=[s[0],(if false then t[1] else (if true then u else s[0])),true];let a2=[t[1],(if false then s[0] else u)];u=[true];}",
        [("NX-E1303", None, 116, 117),
         ("NX-E0102", "type_mismatch", 174, 175)],
    ),
    "later_union_deferred_conflicts_report_first_only": (
        "fn Main(){let x=1;var v1=[];var v2=[];var v3=[];let a0=[v2,(if false then v2[1] else (if true then v3 else v1[0]))];let a1=[v2[1],v2[x],(if false then v2[1] else (if true then v3 else v1[0]))];v3=[];v2=[true];}",
        [("NX-E0102", "type_mismatch", 74, 79),
         ("NX-E1303", None, 176, 178)],
    ),
    "synthesized_union_does_not_lend_projection_anchor": (
        "fn Main(){var q=[];var r=[];var s=[];let a0=[(if true then (if true then [r] else s) else (if true then r else q[0]))];q=[1];}",
        [("NX-E0103", None, 25, 27)],
    ),
}

# The second literal in the middle case must select its own first operand.
EXPECTED_REL = {
    "first_literal_operand_after_completion": {0: (36, 40)},
    "first_operand_of_second_literal": {1: (45, 49)},
    "first_literal_operand_with_index_variable": {0: (44, 48)},
    "literal_origin_after_prior_if": {0: (81, 85)},
    "literal_origin_after_nested_array": {0: (71, 75)},
    "literal_origin_after_if_projection": {0: (90, 94)},
    "literal_origin_after_two_literals": {0: (90, 94)},
    "later_union_branch_preserves_deferred_equality": {0: (92, 96), 1: (144, 148)},
    "later_union_deferred_conflicts_report_first_only": {0: (56, 58), 1: (151, 156)},
}

# D13: these two sources require ResultStatus/JoinRecord for an if used as
# a literal operand. Each is exercised in two profiles and by check/run:
# eight invocations move together to C3.
C3_OPERAND_IF = frozenset({
    "deferred_operand_origin_before_assignment",
    "failed_nested_if_no_retroactive_index",
})


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
    parser.add_argument("--commit", choices=("C2", "C3"), default="C3")
    args = parser.parse_args()
    report = {"schema": "i07c-c2-discriminators-v2", "complete": True,
              "commit": args.commit,
              "candidate_sha256": hashlib.sha256(args.candidate_exe.read_bytes()).hexdigest(),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "cases": {}, "failures": []}
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1"):
        for name, (source, expected) in CASES.items():
            if args.commit == "C2" and name in C3_OPERAND_IF:
                continue
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
