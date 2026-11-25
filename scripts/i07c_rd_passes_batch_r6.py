#!/usr/bin/env python3
"""Run R6 pass counters sequentially while holding the shared heavy lock."""
import subprocess
import sys
import json
from pathlib import Path

from i07c_rd_drive_r6 import CORPORA, CANDIDATE, free_virtual_memory_kb


def main():
    for corpus in CORPORA:
        artifact = Path("docs/validation/i07c-rediseno") / f"r6-pasadas-{corpus}.json"
        if artifact.exists() and not json.loads(artifact.read_text(encoding="utf-8"))["inconclusive"]:
            print(f"reuse {corpus}", flush=True)
            continue
        free = free_virtual_memory_kb()
        if free <= 25 * 1024 * 1024:
            raise RuntimeError(f"FreeVirtualMemory <= 25 GiB before {corpus}: {free}")
        print(f"start {corpus}", flush=True)
        result = subprocess.run([sys.executable, "-B", "scripts/i07c_rd_passes_r6.py",
                                 corpus, CANDIDATE], check=False)
        if result.returncode:
            raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
