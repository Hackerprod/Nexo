#!/usr/bin/env python3
"""Measure a narrower non-T5 branch without the two unproven switches."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys

import i07c_rd_sweep as sweep
from i07c_rd_drive6 import BASE, CANDIDATE, FLAGS

OMIT = {"r1t5", "r5t5", "retract", "atomic0"}
NARROW_FLAGS = ",".join(flag for flag in FLAGS.split(",") if flag not in OMIT)
CORPORA = ("b-projections-2026112401", "b-projections-t5-2026112402",
           "b-projections-2026112403")
DEST = Path("docs/validation/i07c-rediseno")


def run(*args):
    result = subprocess.run([sys.executable, "-B", *args], capture_output=True,
                            text=True, check=True)
    return result.stdout


def main():
    candidate_sha = hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest()
    base_sha = hashlib.sha256(Path(BASE).read_bytes()).hexdigest()
    if candidate_sha != "82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb":
        raise RuntimeError("unexpected candidate binary")
    tag = "proto-" + candidate_sha[:12] + "-noT5"
    probes = json.loads(run("scripts/i07c_rd_cheap.py", CANDIDATE, NARROW_FLAGS))
    (DEST / "r5-noT5-sondas.json").write_text(
        json.dumps({"candidate_sha256": candidate_sha, **probes}, ensure_ascii=False,
                   indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"probes": probes["summary"]}), flush=True)
    for corpus in CORPORA:
        source_count = len(sweep.data(corpus))
        batch = json.loads(run("scripts/i07c_rd_sweep.py", "run", tag, corpus, CANDIDATE,
                               "--flags", NARROW_FLAGS, "--start", "0",
                               "--end", str(source_count)))
        if batch["errors"]:
            raise RuntimeError(f"INCONCLUSO {corpus}: {batch['errors']} process errors")
        comparison = json.loads(run("scripts/i07c_rd_sweep.py", "table",
                                    "main-" + base_sha[:12], tag, corpus))
        comparison.update({"base_sha256": base_sha, "candidate_sha256": candidate_sha,
                           "flags": NARROW_FLAGS})
        (DEST / f"r5-noT5-summary-{corpus}.json").write_text(
            json.dumps(comparison, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"corpus": corpus, "core": comparison["core"],
                          "full": comparison["full"]}), flush=True)


if __name__ == "__main__":
    main()
