#!/usr/bin/env python3
"""Resume the disposable I07c sweep one published, locked lot at a time."""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

import i07c_rd_sweep as sweep
import i07c_rd_campaign as campaign

ROOT = Path(__file__).resolve().parent.parent
STATE = Path("C:/projects/Nexo/.scratch/sol-rediseno-estado.md")
BASE = "C:/nexo-work/target-rd-main/release/nexo.exe"
PROTO = str(ROOT / ".scratch/rd-bin/txmarksproj.exe")
PROFILE = "i07c-rediseno"
SUMMARY = Path("docs/validation/i07c-rediseno/barrido")


def lot_complete(corpus, start, end, variants, base_tag, proto_tag):
    filename = f"{corpus}-{start:05d}-{end:05d}.json"
    checks = [(base_tag, None)] + [
        (proto_tag + name, campaign.BASE_FLAGS + "," + campaign.VARIANTS[name])
        for name in variants.split(",")
    ]
    for tag, flags in checks:
        path = sweep.OUT / tag / filename
        if not path.exists():
            return False
        doc = json.loads(path.read_text(encoding="utf-8"))
        if doc.get("start") != start or doc.get("end") != end or doc.get("flags") != flags or len(doc.get("results", [])) != end - start:
            return False
        if any(isinstance(value, dict) for value in doc["results"]):
            return False
    summary_path = ROOT / SUMMARY / f"summary-{corpus}.json"
    if not summary_path.exists():
        return False
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    return all(summary.get("variants", {}).get(name, {}).get("n", 0) >= end
               for name in variants.split(","))


def free_virtual_kb():
    command = "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory"
    output = subprocess.check_output(["powershell", "-NoProfile", "-Command", command], text=True)
    return int(output.strip())


def memory_ready():
    deadline = time.monotonic() + 20 * 60
    while True:
        free_gb = free_virtual_kb() / 1048576
        print(f"free_virtual_gb={free_gb:.2f}", flush=True)
        if free_gb > 25:
            return True
        if time.monotonic() >= deadline:
            STATE.open("a", encoding="utf-8").write(
                f"{datetime.now().astimezone().isoformat()} | barrido detenido: memoria virtual libre "
                f"{free_gb:.2f} GB tras 20 min; resultados parciales conservados | "
                f"proto {subprocess.check_output(['git', 'rev-parse', '--short', 'HEAD'], text=True).strip()}\n"
            )
            return False
        time.sleep(120)


def run_locked(corpus, start, end, variants):
    command = [sys.executable, "-B", "scripts/heavy-lock.py", "run", "--session", PROFILE,
               "--", sys.executable, "-B", "scripts/i07c_rd_campaign.py", corpus, BASE,
               PROTO, "--variants", variants, "--start", str(start), "--end", str(end)]
    proc = subprocess.Popen(command, cwd=ROOT)
    try:
        return proc.wait(timeout=19 * 60)
    except subprocess.TimeoutExpired:
        # The wrapper and all descendants were launched by this process.
        subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"], check=False)
        proc.wait()
        return "INCONCLUSIVE:lot_over_19min"


def publish(corpus, start, end):
    summary = SUMMARY / f"summary-{corpus}.json"
    data = json.loads((ROOT / summary).read_text(encoding="utf-8"))
    print("lot_result=" + json.dumps({name: value["core"] for name, value in data["variants"].items()}), flush=True)
    subprocess.run(["git", "add", "--", str(summary)], cwd=ROOT, check=True)
    staged = subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=ROOT)
    if staged.returncode:
        subprocess.run(["git", "commit", "-m", f"I07c: publicar lote {corpus} {start}-{end} del barrido",
                        "-m", "Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>",
                        "-m", "Claude-Session: https://claude.ai/code/session_01KQGgKzBo7zyj2Ygyks9B3U"],
                       cwd=ROOT, check=True)
        subprocess.run(["git", "push", "origin", "i07c-rediseno-proto"], cwd=ROOT, check=True)
    head = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, text=True).strip()
    STATE.open("a", encoding="utf-8").write(
        f"{datetime.now().astimezone().isoformat()} | lote {corpus} {start}..{end} "
        f"publicado; A core=" + json.dumps({name: value["core"]["A"] for name, value in data["variants"].items()})
        + f" | proto {head}\n"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpora", nargs="+", help="Names accepted by i07c_rd_sweep.py")
    parser.add_argument("--batch", type=int, default=2000)
    parser.add_argument("--variants", default="right,concrete,statement,occurrence,statement_right,occurrence_right,occurrence_unify,projection")
    args = parser.parse_args()
    with open(BASE, "rb") as stream:
        base_tag = "main-" + hashlib.file_digest(stream, "sha256").hexdigest()[:12]
    with open(PROTO, "rb") as stream:
        proto_tag = "proto-" + hashlib.file_digest(stream, "sha256").hexdigest()[:12] + "-"
    for corpus in args.corpora:
        count = len(sweep.data(corpus))
        for start in range(0, count, args.batch):
            end = min(start + args.batch, count)
            if lot_complete(corpus, start, end, args.variants, base_tag, proto_tag):
                print(f"already_published={corpus} {start}:{end}", flush=True)
                continue
            print(f"starting={corpus} {start}:{end}", flush=True)
            while True:
                if not memory_ready():
                    return 2
                code = run_locked(corpus, start, end, args.variants)
                if code == 75:
                    print("heavy_lock_busy: retrying in 60 seconds", flush=True)
                    time.sleep(60)
                    continue
                if code != 0:
                    print(f"INCONCLUSIVE {corpus} {start}:{end} exit={code}", flush=True)
                    return 1
                publish(corpus, start, end)
                print("cooldown=150s", flush=True)
                time.sleep(150)
                break
    return 0


if __name__ == "__main__":
    sys.exit(main())
