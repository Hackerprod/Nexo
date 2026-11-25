#!/usr/bin/env python3
"""Wait for the shared heavy lock, then run one bounded sequential sweep."""

import os
import subprocess
import sys
import time

from i07c_rd_drive6 import free_virtual_memory_kb


def main():
    env = os.environ.copy()
    env["I07C_RD_LOCK_HELD"] = "1"
    if sys.argv[1:] == ["--followup"]:
        work = ["scripts/i07c_rd_followup_work.py"]
    elif sys.argv[1:] == ["--no-t5"]:
        work = ["scripts/i07c_rd_no_t5.py"]
    else:
        work = ["scripts/i07c_rd_drive6.py", *sys.argv[1:]]
    command = [sys.executable, "-B", "scripts/heavy-lock.py", "run",
               "--session", "i07c-rediseno", "--", sys.executable, "-B",
               *work]
    low_memory_since = None
    while True:
        if free_virtual_memory_kb() <= 25 * 1024 * 1024:
            low_memory_since = low_memory_since or time.monotonic()
            if time.monotonic() - low_memory_since >= 20 * 60:
                raise RuntimeError("free virtual memory <= 25 GiB for 20 min")
            print("free virtual memory <= 25 GiB; retry in 120 s", flush=True)
            time.sleep(120)
            continue
        low_memory_since = None
        result = subprocess.run(command, env=env, capture_output=True, text=True)
        if result.returncode == 0:
            print(result.stdout, end="", flush=True)
            return
        if "trabajo pesado ocupado" not in result.stdout + result.stderr:
            print(result.stdout, end="", flush=True)
            print(result.stderr, end="", file=sys.stderr, flush=True)
            raise SystemExit(result.returncode)
        print("heavy lock occupied; retry in 60 s", flush=True)
        time.sleep(60)


if __name__ == "__main__":
    main()
