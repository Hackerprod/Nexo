#!/usr/bin/env python3
"""Publish R5 acceptance checks one locked, resumable lot at a time."""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

import i07c_rd_sweep as sweep
from i07c_rd_campaign import BASE_FLAGS

ROOT = Path(__file__).resolve().parent.parent
STATE = Path("C:/projects/Nexo/.scratch/sol-rediseno-estado.md")
FLAGS = BASE_FLAGS + ",r5prefix,r1t5,r5t5"


def free_virtual_kb():
    command = "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory"
    return int(subprocess.check_output(["powershell", "-NoProfile", "-Command", command], text=True).strip())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpora", nargs="+")
    parser.add_argument("--batch", type=int, default=2000)
    parser.add_argument("--base", required=True)
    parser.add_argument("--proto", required=True)
    parser.add_argument("--mode", choices=("debug", "release"), default="release")
    parser.add_argument("--series", default="r5")
    args = parser.parse_args()
    binaries = {"base": str(Path(args.base).resolve()), "proto": str(Path(args.proto).resolve())}
    hashes = {role: hashlib.sha256(Path(exe).read_bytes()).hexdigest() for role, exe in binaries.items()}
    for corpus in args.corpora:
        rows = sweep.data(corpus)
        for start in range(0, len(rows), args.batch):
            end = min(start + args.batch, len(rows))
            output = ROOT / "docs/validation/i07c-rediseno/aceptacion" / (
                f"{args.series}-{corpus}-{args.mode}-{start:05d}-{end:05d}.json")
            if output.exists():
                doc = json.loads(output.read_text(encoding="utf8"))
                checks = doc.get("checks", {})
                if (doc.get("hashes") == hashes and doc.get("flags") == FLAGS
                        and len(checks) == 6 and all(not row["inconclusive"] for row in checks.values())):
                    print(f"cached={output}", flush=True)
                    continue
            print(f"starting={corpus} {start}:{end} {args.mode}", flush=True)
            deadline = time.monotonic() + 20 * 60
            while True:
                free = free_virtual_kb() / 1048576
                disk = shutil.disk_usage("C:/").free / 1024**3
                print(f"free_virtual_gb={free:.2f} free_disk_gb={disk:.2f}", flush=True)
                if disk < 12 or (free <= 25 and time.monotonic() >= deadline):
                    print("INCONCLUSIVE: machine resources", flush=True)
                    return 2
                if free <= 25:
                    time.sleep(60)
                    continue
                command = [sys.executable, "-B", "scripts/heavy-lock.py", "run", "--session",
                           "i07c-rediseno", "--", sys.executable, "-B", "scripts/i07c_rd_accept.py",
                           corpus, binaries["base"], binaries["proto"], "--flags", FLAGS,
                           "--start", str(start), "--end", str(end), "--mode", args.mode,
                           "--series", args.series]
                wrapper = subprocess.Popen(command, cwd=ROOT)
                try:
                    code = wrapper.wait(timeout=19 * 60)
                except subprocess.TimeoutExpired:
                    subprocess.run(["taskkill", "/PID", str(wrapper.pid), "/T", "/F"], check=False)
                    wrapper.wait()
                    print("INCONCLUSIVE: wrapper exceeded 19 min", flush=True)
                    return 3
                if code == 75:
                    print("heavy_lock_busy: retrying in 60 seconds", flush=True)
                    time.sleep(60)
                    continue
                if code != 0:
                    print(f"INCONCLUSIVE: lot exit {code}", flush=True)
                    return code
                break
            doc = json.loads(output.read_text(encoding="utf8"))
            changes = {key: len(row["acceptance_changes"]) for key, row in doc["checks"].items()}
            internal = {key: len(row["internal"]) for key, row in doc["checks"].items()}
            if len(doc["checks"]) != 6 or any(row["inconclusive"] for row in doc["checks"].values()):
                print("INCONCLUSIVE: incomplete output", flush=True)
                return 4
            subprocess.run(["git", "add", "--", str(output.relative_to(ROOT))], cwd=ROOT, check=True)
            subprocess.run(["git", "commit", "-m", f"I07c: aceptación R5 {corpus} {start}-{end} {args.mode}",
                            "-m", "Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>",
                            "-m", "Claude-Session: https://claude.ai/code/session_01KQGgKzBo7zyj2Ygyks9B3U"],
                           cwd=ROOT, check=True)
            subprocess.run(["git", "push", "origin", "i07c-rediseno-proto"], cwd=ROOT, check=True)
            head = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, text=True).strip()
            STATE.open("a", encoding="utf8").write(
                f"{datetime.now().astimezone().isoformat()} | aceptación R5 {corpus} {start}:{end} "
                f"{args.mode} cambios={changes} internos={internal} | proto {head}\n")
            print(f"completed={corpus} {start}:{end} changes={changes} internal={internal}", flush=True)
            time.sleep(150)
    return 0


if __name__ == "__main__":
    sys.exit(main())
