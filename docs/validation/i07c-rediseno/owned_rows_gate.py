#!/usr/bin/env python3
"""Execute every owned R6 row at a commit, including the F06 cross gate.

Run alongside candidate_gate.py. A green result requires the complete ID lists;
there is deliberately no sampling or ID filter.
"""

import argparse
import hashlib
import json
from pathlib import Path

from candidate_gate import execute, project_main

HERE = Path(__file__).resolve().parent
FAMILIES = (
    ("directed74_normative_ids", "r8-directed74.json"),
    ("matrix45_normative_ids", "r8-matrix45.json"),
    ("frozen13_normative_ids", "r8-directed74.json"),
)
F06_MAIN = [
    ["NX-E0103", None, [27, 29], "", "", None, None],
    ["NX-E1303", None, [83, 85], "I64", "Array<I64>", [51, 55], [83, 85]],
]


def read(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def signature(output):
    return [output["outcome"]["kind"], project_main(output)]


def matrix_signature(row):
    diagnostics = [[d["code"], d["subcause"], d["primary"],
                    d["expected_type"], d["found_type"],
                    d["rel_expected"], d["rel_found"]] for d in row["expected"]]
    return ["rejected" if diagnostics else "checked", diagnostics]


def direct_signature(output):
    return [[d[0], *d[2]] for d in project_main(output)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--commit", choices=[f"C{i}" for i in range(1, 7)], required=True)
    parser.add_argument("--previous", type=Path, help="previous owned_rows_gate report (C2 requires C1)")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    gates = read("r9-commit-gates.json")
    rows = {name: {row["id"]: row for row in read(filename)["rows"]}
            for name, filename in FAMILIES}
    counts = {}
    for family, _ in FAMILIES:
        ids = gates["commits"][args.commit][family]
        assert len(ids) == len(set(ids)) and set(ids) <= rows[family].keys(), family
        counts[family] = len(ids)
    assert set(rows["frozen13_normative_ids"]) >= set(gates["owner_frozen13"])
    prior = None
    if args.previous:
        prior = json.loads(args.previous.read_text(encoding="utf-8"))
        assert prior["complete"] and not prior["failures"]
        assert prior["commit"] == f"C{int(args.commit[1:]) - 1}"
        assert prior["main_sha256"] == sha(args.main_exe)
        assert prior["runner_sha256"] == sha(Path(__file__))
    if args.commit == "C2" and prior is None:
        parser.error("C2 requires --previous C1 to check F06 oracle-to-main regression")
    report = {"schema": "i07c-r16-owned-rows-v1", "commit": args.commit, "complete": True,
              "main_sha256": sha(args.main_exe), "candidate_sha256": sha(args.candidate_exe),
              "runner_sha256": sha(Path(__file__)), "counts": counts,
              "f06_class": None, "failures": []}
    cache = {}

    def run(binary, row):
        assert hashlib.sha256(row["source"].encode("ascii")).hexdigest() == row["source_sha256"]
        key = (binary.resolve(), row["profile"], row["source"])
        if key not in cache:
            cache[key] = execute(binary, row["source"], row["profile"], args.case)
        return cache[key]

    for family, _ in FAMILIES:
        for ident in gates["commits"][args.commit][family]:
            row = rows[family][ident]
            try:
                output = run(args.candidate_exe, row)
                if family == "matrix45_normative_ids":
                    ok = signature(output) == matrix_signature(row)
                else:
                    expected = row["expected_code_primary"]
                    ok = (output["outcome"]["kind"] == ("rejected" if expected else "checked")
                          and direct_signature(output) == expected)
                    if ok:
                        # Fields absent from the directed fixture remain fixed to main
                        # wherever the corresponding diagnostic already exists there.
                        main_diagnostics = project_main(run(args.main_exe, row))
                        candidate_diagnostics = project_main(output)
                        for index, item in enumerate(expected):
                            if index < len(main_diagnostics) and [main_diagnostics[index][0], *main_diagnostics[index][2]] == item:
                                if candidate_diagnostics[index][1] != main_diagnostics[index][1] or candidate_diagnostics[index][3:] != main_diagnostics[index][3:]:
                                    ok = False
                if not ok:
                    report["failures"].append(f"{family}:{ident}: expected={row.get('expected', row.get('expected_code_primary'))!r} observed={signature(output)!r}")
            except Exception as exc:
                report["failures"].append(f"{family}:{ident}: {type(exc).__name__}: {exc}")

    if args.commit in ("C1", "C2"):
        row = rows["matrix45_normative_ids"]["F06"]
        try:
            main_output = run(args.main_exe, row)
            candidate_output = run(args.candidate_exe, row)
            m, c, o = signature(main_output), signature(candidate_output), matrix_signature(row)
            # Freeze the historical diagnostic positions as well as the ADR oracle.
            expected_main = ["rejected", F06_MAIN]
            if m != expected_main:
                report["failures"].append(f"F06: main fixture changed: {m!r}")
            label = "oracle" if c == o else "main" if c == m else "other"
            report["f06_class"] = label
            if label == "other":
                report["failures"].append(f"F06: neither full main nor full ADR: {c!r}")
            if prior and prior["f06_class"] == "oracle" and label == "main":
                report["failures"].append("F06: oracle-to-main regression")
        except Exception as exc:
            report["failures"].append(f"F06: {type(exc).__name__}: {exc}")

    args.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(args.commit, counts, "F06", report["f06_class"], "failures", len(report["failures"]))
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
