#!/usr/bin/env python3
"""Apply D12 to frozen B sources and deterministic alpha-renamed twins."""

import argparse
import collections
import concurrent.futures
import hashlib
import json
import re
import threading
from pathlib import Path

import candidate_gate as gate
from audit_families import source
from c2_min_fuzz import classify, sha

HERE = Path(__file__).resolve().parent
FILES = ("b-projections-2026112401.json", "b-projections-t5-2026112402.json",
         "b-projections-2026112403.json")
PROFILE = "f0-0002-b-v2"


def alpha_renamed(program):
    names = set(re.findall(r"\b(?:let|var)\s+([A-Za-z_][A-Za-z_0-9]*)", program))
    assert names
    renamed = re.sub(r"\b[A-Za-z_][A-Za-z_0-9]*\b",
                     lambda match: "b_" + match.group() if match.group() in names
                     else match.group(), program)
    assert renamed != program
    return renamed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-exe", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--candidate-debug-exe", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not 1 <= args.workers <= 3:
        parser.error("1–3 workers required")
    tasks = []
    corpus_sha = {}
    for filename in FILES:
        path = HERE / filename
        corpus_sha[filename] = sha(path)
        rows = json.loads(path.read_text(encoding="utf-8"))
        assert len(rows) == 3000
        for index, row in enumerate(rows):
            program = source(row)
            tasks.append((filename, index, "original", PROFILE, program))
            tasks.append((filename, index, "alpha_renamed", PROFILE,
                          alpha_renamed(program)))
    assert len(tasks) == 18000
    unique_sources = len({task[4] for task in tasks})
    assert unique_sources >= 15000
    counts = collections.Counter()
    deltas = []
    hard = []

    def evaluate(task):
        filename, index, variant, profile, program = task
        case = args.case.with_name(f"{args.case.stem}-{threading.get_ident()}.n")
        try:
            main_out = gate.execute(args.main_exe, program, profile, case)
            candidate = gate.execute(args.candidate_exe, program, profile, case)
            debug = gate.execute(args.candidate_debug_exe, program, profile, case)
            if (main_out["outcome"]["kind"] == "driver_error" or
                    candidate["outcome"]["kind"] == "driver_error" or
                    debug["outcome"]["kind"] == "driver_error"):
                return task, "hard_failure", None, None
            if gate.signature(debug) != gate.signature(candidate):
                return task, "debug_release_divergence", None, None
            status, detail = classify(main_out, candidate, program)
            main_summary = [main_out["outcome"]["kind"], [
                [item["code"], item.get("subcause"),
                 item.get("data", {}).get("expected_type"),
                 item.get("data", {}).get("found_type")]
                for item in main_out["diagnostics"]]]
            return task, status, detail, main_summary
        except Exception as error:
            return task, "hard_failure", {"error": repr(error)}, None

    originals = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for ordinal, (task, status, detail, main_summary) in enumerate(
            pool.map(evaluate, tasks), 1
        ):
            filename, index, variant, profile, program = task
            key = (filename, index)
            if variant == "original":
                originals[key] = main_summary
            elif main_summary != originals.pop(key):
                status = "alpha_invariance_failure"
                detail = {"main_summary": main_summary}
            counts[status] += 1
            row = {"corpus": filename, "index": index, "variant": variant,
                   "profile": profile,
                   "source_sha256": hashlib.sha256(program.encode("ascii")).hexdigest(),
                   "status": status, "detail": detail}
            if status != "same" and len(deltas) < 1000:
                deltas.append(row)
            if status not in ("same", "improvement"):
                hard.append(row)
            if ordinal % 3000 == 0:
                print(f"B {ordinal}/{len(tasks)}, improvements={counts['improvement']}, "
                      f"hard={len(hard)}", flush=True)
    assert not originals
    result = {"schema": "i07c-c2-min-b-d12-fuzz-v1", "complete": True,
              "profile_cases": len(tasks), "unique_sources": unique_sources,
              "corpus_sha256": corpus_sha, "profile": PROFILE,
              "variants": ["original", "alpha_renamed"],
              "main_sha256": sha(args.main_exe), "candidate_sha256": sha(args.candidate_exe),
              "candidate_debug_sha256": sha(args.candidate_debug_exe),
              "runner_sha256": sha(Path(__file__)), "counts": dict(counts),
              "deltas": deltas, "hard_failures": hard}
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n",
                           encoding="utf-8")
    print("B", len(tasks), "profile-cases; hard", len(hard))
    if hard:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
