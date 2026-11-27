#!/usr/bin/env python3
"""Fuzz the first operand of adjacent array literals across later bindings.

The narrow mode reconstructs B r24's reported family. Its private fuzz2.py
is unavailable in the validator clone. The varied mode samples independent
AST structures, operand names, projections, nested ifs, and later writes.
"""

import argparse
import collections
import concurrent.futures
import hashlib
import json
import random
import threading
from pathlib import Path

import candidate_gate as gate


def narrow_source(seed):
    rng = random.Random(seed ^ 0xB24)
    first = rng.choice(("s[x]", "s[0]", "t[x]", "t[1]"))
    prior = rng.choice((
        "(if false then s[0] else u)", "(if false then s[x] else u)",
        "(if true then t[x] else u[1])", "[u]", "s[x]", "t[x]",
    ))
    second = rng.choice((
        "(if false then u else u)", "(if true then t[x] else u[1])",
        "(if false then [s] else t)", "s[0]", "t[1]", "[u]",
    ))
    third = rng.choice(("1", "true", "s[0]", "t[1]", "[u]"))
    before = f"let a0=[{prior},{rng.choice(('u', '[u]', 't[1]'))}];"
    middle = f"let a1=[{first},{second},{third}];"
    after = rng.choice(("u=[true];", "u=[1];", "s=[true];", "t=[1];"))
    return "fn Main(){let x=1;var s=[];var t=[];var u=[];" + before + middle + after + "}"


def varied_source(seed):
    rng = random.Random(seed ^ 0xC2A17)
    s, t, u = rng.choice((("s", "t", "u"), ("alpha", "beta", "gamma"),
                          ("left", "right", "tail"), ("v1", "v2", "v3")))
    reads = (f"{s}[x]", f"{s}[0]", f"{t}[1]", f"{t}[x]", f"{u}[0]",
             f"{u}[1]", s, t, u)
    nested = (
        f"(if false then {s}[0] else {u})",
        f"(if true then {t}[x] else {u}[1])",
        f"(if false then [{s}] else {t})",
        f"(if true then (if false then {s}[x] else {u}) else {t}[1])",
        f"(if false then {t}[1] else (if true then {u} else {s}[0]))",
        f"[{u}]", f"[{s}[x]]", "1", "true",
    )
    prior = f"let a0=[{rng.choice(reads + nested)},{rng.choice(reads + nested)}];"
    middle = (f"let a1=[{rng.choice(reads[:6])},"
              f"{rng.choice(reads + nested)},{rng.choice(reads + nested)}];")
    tail = (f"let a2=[{rng.choice(reads[:6])},{rng.choice(reads + nested)}];"
            if rng.randrange(3) else "")
    after = rng.choice((
        f"{u}=[true];", f"{u}=[1];", f"{s}=[true];", f"{t}=[1];",
        f"{u}=[];{t}=[true];", f"{s}=[1];{u}=[true];",
    ))
    return (f"fn Main(){{let x=1;var {s}=[];var {t}=[];var {u}=[];"
            + prior + middle + tail + after + "}")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--candidate-debug-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mode", choices=("narrow", "varied"), default="varied")
    parser.add_argument("--count", type=int, default=60_000)
    parser.add_argument("--workers", type=int, default=3)
    args = parser.parse_args()
    if args.mode == "varied" and args.count < 60_000:
        raise SystemExit("varied evidence requires at least 60000 programs")
    if not 1 <= args.workers <= 3:
        raise SystemExit("workers must be between 1 and 3")
    source = narrow_source if args.mode == "narrow" else varied_source
    profile = "f0-0003-b-v1"
    counts = collections.Counter()
    unique = set()
    examples = []
    hard = []
    def evaluate(seed):
        program = source(seed)
        case_path = args.case.with_name(
            f"{args.case.stem}-{threading.get_ident()}{args.case.suffix}")
        try:
            baseline = gate.execute(args.main_exe, program, profile, case_path)
            candidate = gate.execute(args.candidate_exe, program, profile, case_path)
            debug = gate.execute(args.candidate_debug_exe, program, profile, case_path)
            m, c, d = map(gate.signature, (baseline, candidate, debug))
            flags = []
            if c != d:
                flags.append("debug_release_divergence")
            if any(row["outcome"]["kind"] == "driver_error" for row in (baseline, candidate, debug)):
                flags.append("internal_error")
            if baseline["outcome"]["kind"] != candidate["outcome"]["kind"]:
                flags.append("accept_reject_divergence")
            label = "same" if m == c else gate.classify(baseline, candidate, gate.oracle.derive(program))
            if label == "A":
                flags.append("A")
            return program, label, flags, m, c
        except Exception as error:
            return program, None, [repr(error)], None, None

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
      for seed, (program, label, flags, m, c) in enumerate(pool.map(evaluate, range(args.count))):
        unique.add(program)
        if label is not None:
            counts[label] += 1
            if label in ("A", "D", "A-unmodeled") and len(examples) < 300:
                examples.append({"seed": seed, "source": program, "class": label,
                                 "main": m, "candidate": c})
        for reason in flags:
            if label is None:
                counts["error_or_hang"] += 1
            elif reason != label:
                counts[reason] += 1
            hard.append({"seed": seed, "source": program, "reason": reason,
                         "main": m, "candidate": c})
        if (seed + 1) % 10_000 == 0:
            print(f"{args.mode}: {seed + 1}/{args.count}, unique={len(unique)}, "
                  f"A={counts['A']}, hard={len(hard)}", flush=True)
    report = {
        "schema": "i07c-c2-literal-origin-fuzz-v2", "mode": args.mode,
        "complete": True, "programs": args.count, "unique_sources": len(unique),
        "profile": profile, "main_sha256": sha(args.main_exe),
        "candidate_release_sha256": sha(args.candidate_exe),
        "candidate_debug_sha256": sha(args.candidate_debug_exe),
        "script_sha256": sha(Path(__file__)), "counts": dict(counts),
        "examples": examples, "hard_failures": hard,
    }
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(f"{args.mode}: {args.count} programs, {len(unique)} unique, "
          f"A={counts['A']}, hard={len(hard)}")
    if hard:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
