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
import os
import random
import subprocess
import threading
from pathlib import Path

import candidate_gate as gate
from c2_family_check import classify as causal_classify


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
    parser.add_argument("--trace-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mode", choices=("narrow", "varied"), default="varied")
    parser.add_argument("--count", type=int, default=60_000)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--smoke", action="store_true", help="report-only small run, never gate evidence")
    args = parser.parse_args()
    if args.mode == "varied" and args.count < 60_000 and not args.smoke:
        raise SystemExit("varied evidence requires at least 60000 programs")
    if not 1 <= args.workers <= 3:
        raise SystemExit("workers must be between 1 and 3")
    source = narrow_source if args.mode == "narrow" else varied_source
    profile = "f0-0003-b-v1"
    counts = collections.Counter()
    unique = set()
    examples = []
    hard = []
    family_rows = []

    def trace_family(program, profile, case_path, baseline, candidate):
        case_path.write_bytes(program.encode("utf-8"))
        env = os.environ.copy()
        env["NEXO_ORIGIN_TRACE"] = "1"
        run = subprocess.run([str(args.trace_exe), "check", str(case_path), "--profile", profile,
                              "--format", "json"], capture_output=True, env=env, timeout=30,
                             check=False)
        if run.returncode not in (0, 1):
            raise RuntimeError(f"trace exit {run.returncode}: {run.stderr[:300]!r}")
        if gate.compact_match(json.loads(run.stdout)) != gate.compact_match(candidate):
            raise ValueError("trace feature changed candidate output")
        lines = run.stderr.decode("utf-8", errors="replace").splitlines()
        if any(not line.startswith("C2_ORIGIN\t") for line in lines):
            raise ValueError("unrelated trace stderr")
        events = [json.loads(line.removeprefix("C2_ORIGIN\t")) for line in lines]
        return causal_classify(gate.compact_match(baseline), gate.compact_match(candidate), events)

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
            family, witness = (None, None)
            if label in ("D", "A-unmodeled") and not flags:
                family, witness = trace_family(program, profile, case_path, baseline, candidate)
                if family is None:
                    flags.append("unclassified_difference")
            return program, label, flags, m, c, family, witness
        except Exception as error:
            return program, None, [repr(error)], None, None, None, None

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
      for seed, (program, label, flags, m, c, family, witness) in enumerate(pool.map(evaluate, range(args.count))):
        unique.add(program)
        if label is not None:
            counts[label] += 1
            if label in ("A", "D", "A-unmodeled") and len(examples) < 300:
                examples.append({"seed": seed, "source": program, "class": label,
                                 "main": m, "candidate": c})
        if family is not None:
            counts["family:" + family] += 1
            family_rows.append({"seed": seed, "source_sha256": hashlib.sha256(program.encode("ascii")).hexdigest(),
                                "class": label, "family": family, "witness": witness})
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
        "schema": "i07c-c2-literal-origin-fuzz-v3", "mode": args.mode,
        "complete": not args.smoke, "programs": args.count, "unique_sources": len(unique),
        "profile": profile, "main_sha256": sha(args.main_exe),
        "candidate_release_sha256": sha(args.candidate_exe),
        "candidate_debug_sha256": sha(args.candidate_debug_exe),
        "trace_sha256": sha(args.trace_exe),
        "script_sha256": sha(Path(__file__)), "counts": dict(counts),
        "examples": examples, "family_rows": family_rows, "hard_failures": hard,
    }
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(f"{args.mode}: {args.count} programs, {len(unique)} unique, "
          f"A={counts['A']}, hard={len(hard)}")
    if args.smoke:
        print("REPORT-ONLY SMOKE: never evidence of gate success")
    if hard:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
