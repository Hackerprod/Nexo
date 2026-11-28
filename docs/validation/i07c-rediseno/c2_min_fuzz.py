#!/usr/bin/env python3
"""D12 fuzz of the C2 minimum with a full oracle check for every delta."""

import argparse
import collections
import concurrent.futures
import hashlib
import json
import threading
from pathlib import Path

import candidate_gate as gate
import c2_literal_origin_fuzz as generator
from c2_d12_census import distance, new_false_positives


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def classify(main, candidate, source):
    m, c = gate.signature(main), gate.signature(candidate)
    if m == c:
        return "same", None
    derived = gate.oracle.derive(source)
    complete = not any(derived.get(key) for key in
                       ("error", "unmodeled", "no_fijado", "d_optional"))
    if not complete:
        return "conserve_main", None
    expected = gate.oracle_signature(derived)
    dm, dc = distance(m, expected), distance(c, expected)
    fp = new_false_positives(m, c, expected)
    if dc < dm and fp == 0 and m[0] == c[0]:
        return "improvement", {"distance_main": dm, "distance_candidate": dc}
    return "complete_defect", {"distance_main": dm, "distance_candidate": dc,
                               "new_false_positives": fp}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--candidate-debug-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--count", type=int, default=60100)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--survey", action="store_true",
                        help="frozen 5000-source D12 census instead of the large fuzz")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    minimum = 5000 if args.survey else 60000
    if args.count < minimum or not 1 <= args.workers <= 3:
        parser.error(f"at least {minimum} programs and 1–3 workers are required")
    profile = "f0-0003-b-v1"
    counts = collections.Counter()
    unique = set()
    deltas = []
    hard = []

    def evaluate(seed):
        source = generator.varied_source(seed)
        case = args.case.with_name(f"{args.case.stem}-{threading.get_ident()}.n")
        try:
            main_out = gate.execute(args.main_exe, source, profile, case)
            candidate = gate.execute(args.candidate_exe, source, profile, case)
            debug = gate.execute(args.candidate_debug_exe, source, profile, case)
            if (main_out["outcome"]["kind"] == "driver_error" or
                    candidate["outcome"]["kind"] == "driver_error" or
                    debug["outcome"]["kind"] == "driver_error"):
                return source, "hard_failure", None
            if gate.signature(debug) != gate.signature(candidate):
                return source, "debug_release_divergence", None
            status, detail = classify(main_out, candidate, source)
            return source, status, detail
        except Exception as error:
            return source, "hard_failure", {"error": repr(error)}

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for seed, (source, status, detail) in enumerate(pool.map(evaluate, range(args.count))):
            unique.add(source)
            counts[status] += 1
            if status != "same" and len(deltas) < 1000:
                deltas.append({"seed": seed, "source": source,
                               "source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(),
                               "status": status, "detail": detail})
            if status not in ("same", "improvement"):
                hard.append({"seed": seed, "source_sha256": hashlib.sha256(
                    source.encode("ascii")).hexdigest(), "status": status, "detail": detail})
            if (seed + 1) % 10000 == 0:
                print(f"varied {seed + 1}/{args.count}, unique={len(unique)}, "
                      f"improvements={counts['improvement']}, hard={len(hard)}", flush=True)
    if len(unique) < minimum:
        hard.append({"status": "insufficient_unique_sources", "unique": len(unique)})
    report = {"schema": "i07c-c2-min-d12-fuzz-v1", "complete": True,
              "programs": args.count, "unique_sources": len(unique), "profile": profile,
              "survey": args.survey,
              "main_sha256": sha(args.main_exe), "candidate_sha256": sha(args.candidate_exe),
              "candidate_debug_sha256": sha(args.candidate_debug_exe),
              "generator_sha256": sha(Path(generator.__file__)),
              "runner_sha256": sha(Path(__file__)), "counts": dict(counts),
              "deltas": deltas, "hard_failures": hard}
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("varied", args.count, "unique", len(unique), "hard", len(hard))
    if hard:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
