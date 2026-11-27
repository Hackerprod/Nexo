#!/usr/bin/env python3
"""Reproduce the narrow C2 array operand fuzz after B r23."""

import argparse
import hashlib
import json
import random
from pathlib import Path

import candidate_gate as gate


def source(seed):
    choice = random.Random(seed)
    first = choice.choice(("t[0]", "s[0]", "t[x]", "s[x]"))
    branch = choice.choice((
        "(if true then 1 else s[0])",
        "(if false then s[0] else 1)",
        "(if true then [1] else t)",
        "(if true then 1 else s[x])",
        "(if true then s else t)",
        "[t]", "[s]",
    ))
    last = choice.choice(("s", "t", "[t]", "[s]", "1", "true"))
    later = choice.choice((
        "t=[1];", "t=[];", "t=[true];", "s=[1];", "s=[];",
        "s=[true];", "t=[1];s=[];",
    ))
    prefix = "fn Main(){let x=1;var s=[];var t=[];"
    if seed % 4 == 0:
        return prefix + f"let a0=[{first},[{last}]];let a1=[{first},{branch}];" + later + "}"
    return prefix + f"let a0=[{first},{branch},{last}];" + later + "}"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    profile = "f0-0003-b-v1"
    counts = {}
    failures = []
    cache = {}
    for seed in range(14_000):
        program = source(seed)
        if program in cache:
            label = cache[program]
        else:
            try:
                baseline = gate.execute(args.main_exe, program, profile, args.case)
                candidate = gate.execute(args.candidate_exe, program, profile, args.case)
                if gate.signature(baseline) == gate.signature(candidate):
                    label = "same"
                else:
                    label = gate.classify(baseline, candidate, gate.oracle.derive(program))
            except Exception as error:
                label = "error"
                failures.append({"seed": seed, "source": program, "error": repr(error)})
            cache[program] = label
        counts[label] = counts.get(label, 0) + 1
        if label == "A":
            failures.append({"seed": seed, "source": program, "class": label})
    report = {
        "schema": "i07c-c2-literal-origin-fuzz-v1", "complete": True,
        "programs": 14_000, "unique_sources": len(cache), "profile": profile,
        "main_sha256": sha(args.main_exe), "candidate_sha256": sha(args.candidate_exe),
        "script_sha256": sha(Path(__file__)), "counts": counts, "failures": failures,
    }
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(f"C2 literal origin fuzz: 14000 programs, {len(cache)} unique, "
          f"A={counts.get('A', 0)}, errors={counts.get('error', 0)}")
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
