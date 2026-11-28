#!/usr/bin/env python3
"""Attribute frozen C2 repros to their first observable source commit.

This is an investigation harness, never a PASS gate. It builds each commit that
changes src/ in a detached worktree and evaluates the same frozen sources.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import candidate_gate as gate  # noqa: E402
import c2_d12_census as d12  # noqa: E402


def command(*args, cwd=None, env=None, timeout=1500):
    return subprocess.run(args, cwd=cwd, env=env, capture_output=True,
                          text=True, timeout=timeout, check=True)


def defect_status(main, candidate, expected, complete):
    if candidate == main:
        return "same"
    if not complete:
        return "conserve_main"
    dm = d12.distance(main, expected)
    dc = d12.distance(candidate, expected)
    fp = d12.new_false_positives(main, candidate, expected)
    return "improvement" if dc < dm and fp == 0 and candidate[0] == main[0] else "complete_defect"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--base", default="af19a34")
    parser.add_argument("--head", default="d47a46f")
    parser.add_argument("--limit-commits", type=int)
    args = parser.parse_args()

    frozen = json.loads((HERE / "c2-fuzz-5000-pd6.json").read_text(encoding="utf-8"))
    census = json.loads((HERE / "c2-d12-census-5000.json").read_text(encoding="utf-8"))
    by_seed = {row["seed"]: row for row in census["rows"]}
    wanted_improvements = [row["seed"] for row in census["rows"] if row["status"] == "improvement"][:10]
    wanted = {row["seed"] for row in census["rows"] if row["status"] != "improvement"}
    wanted.update(wanted_improvements)
    rows = {}
    for row in frozen["hard_failures"]:
        seed = row["seed"]
        if seed not in wanted:
            continue
        derived = gate.oracle.derive(row["source"])
        rows[f"fuzz:{seed}"] = {
            "source": row["source"], "profile": frozen["profile"],
            "main": row["main"], "final": row["candidate"],
            "complete": by_seed[seed]["complete"],
            "expected": gate.oracle_signature(derived) if by_seed[seed]["complete"] else None,
            "final_status": by_seed[seed]["status"],
        }
    assert len(rows) == 72, len(rows)
    historical = json.loads((HERE / "c2-final-iteration-v15f.json").read_text(encoding="utf-8"))
    for key, item in historical["rows"].items():
        rows[f"historical:{key}"] = {
            "source": item["source"], "profile": item["profile"],
            "main_compact": historical["results"][key]["main"],
        }
    assert len(rows) == 192, len(rows)
    commits = command("git", "log", "--reverse", "--format=%H", args.base + ".." + args.head,
                      "--", ":/src", cwd=HERE).stdout.splitlines()
    commits.insert(0, command("git", "rev-parse", args.base, cwd=HERE).stdout.strip())
    if args.limit_commits:
        commits = commits[:args.limit_commits]
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(args.target)
    env["CARGO_BUILD_JOBS"] = "4"
    env["CARGO_INCREMENTAL"] = "1"
    case = args.target / "bisect-case.nx"
    case.parent.mkdir(parents=True, exist_ok=True)
    report = {"schema": "i07c-c2-commit-bisect-v1", "base": args.base,
              "head": args.head, "source_commit_count": len(commits),
              "frozen_fuzz_sha256": hashlib.sha256((HERE / "c2-fuzz-5000-pd6.json").read_bytes()).hexdigest(),
              "frozen_census_sha256": hashlib.sha256((HERE / "c2-d12-census-5000.json").read_bytes()).hexdigest(),
              "rows": {key: {"first_difference": None, "first_defect": None,
                              "first_final_output": None, "final_status": row.get("final_status")}
                       for key, row in rows.items()}, "commits": []}
    for ordinal, commit in enumerate(commits, 1):
        started = time.monotonic()
        command("git", "checkout", "--detach", "--force", commit, cwd=args.worktree, timeout=60)
        build = command("cargo", "build", "--release", "--locked", "--offline", "-j", "4",
                        cwd=args.worktree, env=env)
        binary = args.target / "release" / "nexo.exe"
        commit_counts = {"first_difference": 0, "first_defect": 0,
                         "first_final_output": 0, "d12_defects": 0,
                         "d12_improvements": 0, "historical_differences": 0}
        for key, row in rows.items():
            observed = gate.execute(binary, row["source"], row["profile"], case)
            compact = gate.compact_match(observed)
            if key.startswith("historical:"):
                differs = compact != row["main_compact"]
                defect = False
                final_output = False
                if differs:
                    commit_counts["historical_differences"] += 1
            else:
                signature = gate.signature(observed)
                differs = signature != row["main"]
                status = defect_status(row["main"], signature, row["expected"], row["complete"])
                defect = status in ("conserve_main", "complete_defect")
                final_output = signature == row["final"]
                if defect:
                    commit_counts["d12_defects"] += 1
                if status == "improvement":
                    commit_counts["d12_improvements"] += 1
            for name, matches in (("first_difference", differs), ("first_defect", defect),
                                  ("first_final_output", final_output)):
                if matches and report["rows"][key][name] is None:
                    report["rows"][key][name] = commit
                    commit_counts[name] += 1
        report["commits"].append({"commit": commit, "ordinal": ordinal,
                                  "elapsed_seconds": round(time.monotonic() - started, 2),
                                  "counts": commit_counts,
                                  "build_stderr_tail": build.stderr[-200:]})
        args.output.write_bytes((json.dumps(report, ensure_ascii=True, indent=2) + "\n").encode("utf-8"))
        print(f"{ordinal}/{len(commits)} {commit[:8]} {commit_counts} "
              f"{time.monotonic() - started:.1f}s", flush=True)


if __name__ == "__main__":
    main()
