#!/usr/bin/env python3
"""One locked, bounded follow-up batch for rejected R5."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from i07c_rd_drive6 import CANDIDATE, FLAGS

DEST = Path("docs/validation/i07c-rediseno")


def call(args, name, env=None):
    result = subprocess.run([sys.executable, "-B", *args],
                            capture_output=True, text=True, env=env)
    log = DEST / f"r5-{name}.log"
    log.write_text(result.stdout + result.stderr, encoding="utf-8")
    digest = hashlib.sha256(log.read_bytes()).hexdigest()
    print(json.dumps({"step": name, "exit": result.returncode,
                      "log_sha256": digest,
                      "last_lines": (result.stdout + result.stderr).splitlines()[-3:]}),
          flush=True)
    return {"exit": result.returncode, "log_sha256": digest,
            "last_lines": (result.stdout + result.stderr).splitlines()[-3:]}


def main():
    candidate_sha = hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest()
    if candidate_sha != "82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb":
        raise RuntimeError("R5 candidate binary changed")
    started = time.monotonic()
    env = os.environ.copy()
    env["NEXO_XV"] = FLAGS
    result = {"candidate_sha256": candidate_sha, "flags": FLAGS, "steps": {}}
    result["steps"]["rd152"] = call(
        ["scripts/i07c_rd_conformidad.py", "--verify", CANDIDATE], "rd152", env)
    for piece in ("retract", "atomic0"):
        if time.monotonic() - started > 16 * 60:
            print("bounded batch reached 16 min; remaining ablations need another lock", flush=True)
            break
        result["steps"][piece] = call(
            ["scripts/i07c_rd_find_ablation.py", CANDIDATE, piece, "--limit", "300"],
            "ablation-" + piece)
    dest = DEST / "r5-followup-summary.json"
    dest.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
