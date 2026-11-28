#!/usr/bin/env python3
"""Reduce one frozen D12 witness per M1–M5 by deleting whole statements.

The output is a checklist for C3, not a replacement for the complete fuzz gate.
Each accepted reduction is rerun against main, the historical C2 candidate,
and the ADR oracle; it must retain its original D12 failure class. The
historical candidate may have a different relation span than the earlier
frozen fuzz binary, so its complete JSON is not borrowed from that report.
"""

import argparse
import hashlib
import json
from pathlib import Path

import candidate_gate as gate
from c2_d12_census import distance, new_false_positives

HERE = Path(__file__).resolve().parent
SEEDS = {"M1": 158, "M2": 306, "M3": 830, "M4": 1124, "M5": 520}
INTRODUCER = {"M1": "2841cc6", "M2": "2841cc6", "M3": "f5b12b1",
              "M4": "f5b12b1", "M5": "2841cc6"}


def sha(value):
    return hashlib.sha256(value.encode("ascii")).hexdigest()


def parts(source):
    assert source.startswith("fn Main(){") and source.endswith("}")
    body = source[len("fn Main(){"):-1]
    return [item + ";" for item in body.split(";") if item]


def check(source, family, main, candidate, case):
    if "(if " not in source:
        return None
    try:
        derived = gate.oracle.derive(source)
        if derived.get("error") or derived.get("unmodeled"):
            return None
        m = gate.execute(main, source, "f0-0003-b-v1", case)
        c = gate.execute(candidate, source, "f0-0003-b-v1", case)
        ms, cs = gate.signature(m), gate.signature(c)
        if ms == cs or any(d[0] == "NX-E0002" for d in ms[1] + cs[1]):
            return None
        complete = not any(derived.get(key) for key in ("no_fijado", "d_optional"))
        if family == "M1":
            if complete or not derived.get("d_optional"):
                return None
            status = "conserve_main"
        else:
            if not complete:
                return None
            expected = gate.oracle_signature(derived)
            dm, dc = distance(ms, expected), distance(cs, expected)
            fp = new_false_positives(ms, cs, expected)
            if fp < 1:
                return None
            status = "complete_defect"
        return {"status": status, "main": ms, "candidate": cs,
                "oracle_complete": complete}
    except (RuntimeError, ValueError, KeyError):
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--p1d-exe", type=Path, required=True)
    parser.add_argument("--join-exe", type=Path, required=True)
    parser.add_argument("--minimum-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    frozen = json.loads((HERE / "c2-fuzz-5000-pd6.json").read_text(encoding="utf-8"))
    census = json.loads((HERE / "c2-d12-census-5000.json").read_text(encoding="utf-8"))
    status_by_seed = {row["seed"]: row["status"] for row in census["rows"]}
    source_by_seed = {row["seed"]: row for row in frozen["hard_failures"]}
    result = {"schema": "i07c-c2-d13-reductions-v1",
              "main_sha256": hashlib.sha256(args.main_exe.read_bytes()).hexdigest(),
              "minimum_sha256": hashlib.sha256(args.minimum_exe.read_bytes()).hexdigest(),
              "frozen_fuzz_sha256": hashlib.sha256((HERE / "c2-fuzz-5000-pd6.json").read_bytes()).hexdigest(),
              "rows": {}}
    for family, seed in SEEDS.items():
        frozen_row = source_by_seed[seed]
        source = frozen_row["source"]
        historical = args.p1d_exe if INTRODUCER[family] == "2841cc6" else args.join_exe
        baseline = check(source, family, args.main_exe, historical, args.case)
        if (baseline is None or baseline["main"] != frozen_row["main"]
                or baseline["status"] != status_by_seed[seed]):
            raise RuntimeError(f"{family}: original source does not retain frozen D12 class")
        statements = parts(source)
        changed = True
        while changed:
            changed = False
            for index in range(len(statements)):
                trial = "fn Main(){" + "".join(statements[:index] + statements[index + 1:]) + "}"
                if check(trial, family, args.main_exe, historical, args.case):
                    statements.pop(index)
                    changed = True
                    break
        reduced = "fn Main(){" + "".join(statements) + "}"
        observed = check(reduced, family, args.main_exe, historical, args.case)
        assert observed is not None and len(reduced) < len(source), family
        minimum = gate.signature(gate.execute(args.minimum_exe, reduced,
                                              "f0-0003-b-v1", args.case))
        m = observed["main"]
        if minimum == m:
            minimum_status = "conserve_main"
        elif family != "M1":
            expected = gate.oracle_signature(gate.oracle.derive(reduced))
            minimum_status = ("improvement" if distance(minimum, expected) < distance(m, expected)
                              and new_false_positives(m, minimum, expected) == 0
                              and minimum[0] == m[0] else "defect")
        else:
            minimum_status = "defect"
        if minimum_status == "defect":
            raise RuntimeError(f"{family}: reduction also fails in the minimum")
        result["rows"][family] = {"seed": seed, "original_bytes": len(source),
                                  "original_sha256": sha(source), "source": reduced,
                                  "bytes": len(reduced), "source_sha256": sha(reduced),
                                  "introductory_commit": INTRODUCER[family],
                                  "historical_sha256": hashlib.sha256(historical.read_bytes()).hexdigest(),
                                  "minimum": minimum, "minimum_status": minimum_status,
                                  **observed}
        print(f"{family}: {len(source)} -> {len(reduced)} bytes")
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")


if __name__ == "__main__":
    main()
