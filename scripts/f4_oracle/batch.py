"""Resumable batches of seeds under the shared heavy lock (class ARNES).

Each batch is one ``cli gate`` run over a block of seeds, launched through
``scripts/heavy-lock.py run`` so that it holds the exclusive lock while it works; between batches
the lock is released for at least 60 seconds.  A batch whose output already ends with a summary of
exit 0 is skipped (resume).  Only the processes launched here are waited on; nothing is killed
by name.
"""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import time

from .generic import make_batches

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "scripts" / "heavy-lock.py"
MIN_GAP = 60.0


def validate_scale(gap_seconds: float, output_dir: Path) -> None:
    if gap_seconds < MIN_GAP:
        raise ValueError("heavy-lock gap must be at least 60 seconds")
    if Path(output_dir).resolve().drive.upper() != "C:":
        raise ValueError("output-dir must be on C:")


def plan(*, command: str, binary_debug, binary_release, profile: str, phase: str, complete: str,
         seeds: str, valid: int, invalid: int, output_dir, width: int = 1, session: str = "f4-oracle",
         final: bool = False, timeout: float = 10.0) -> list:
    """One entry per batch: label, seeds, output file and the full command line."""
    out = []
    for group in make_batches(seeds, width):
        label = f"{group[0]:02d}-{group[-1]:02d}"
        output = Path(output_dir) / f"batch-{phase}-{label}.jsonl"
        cmd = [sys.executable, "-B", str(LOCK), "run", "--session", f"{session}-{label}", "--",
               sys.executable, "-B", "-m", "scripts.f4_oracle.cli", command,
               "--binary-debug", str(Path(binary_debug).resolve()),
               "--binary-release", str(Path(binary_release).resolve()),
               "--profile", profile, "--phase", phase, "--complete", complete,
               "--seeds", f"{group[0]}:{group[-1]}", "--valid-per-seed", str(valid),
               "--invalid-per-seed", str(invalid), "--output", str(output),
               "--timeout", str(timeout), "--resume"]
        if final:
            cmd.append("--final")
        out.append({"label": label, "seeds": group, "output": output, "command": cmd})
    return out


def finished(output: Path) -> bool:
    """True when the batch output ends with a summary of exit 0."""
    try:
        lines = Path(output).read_bytes().decode("utf-8").split("\n")
    except OSError:
        return False
    for line in reversed(lines):
        if line.strip():
            try:
                rec = json.loads(line)
            except ValueError:
                return False
            return rec.get("type") == "summary" and rec.get("exit") == 0
    return False


def run(batches: list, *, gap_seconds: float, continue_on_divergence: bool = False,
        dry_run: bool = False, runner=subprocess.run, sleep=time.sleep) -> tuple:
    """Run the batches in order; returns (exit code, report list)."""
    report = []
    worst = 0
    for position, batch in enumerate(batches):
        if finished(batch["output"]):
            report.append({"batch": batch["label"], "skipped": True})
            continue
        if dry_run:
            report.append({"batch": batch["label"], "command": batch["command"]})
            continue
        Path(batch["output"]).parent.mkdir(parents=True, exist_ok=True)
        started = time.monotonic()
        result = runner(batch["command"], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
        report.append({"batch": batch["label"], "exit": result.returncode,
                       "seconds": round(time.monotonic() - started, 3)})
        worst = max(worst, result.returncode)
        if result.returncode and not (continue_on_divergence and result.returncode == 1):
            return result.returncode, report
        if position + 1 < len(batches):
            sleep(gap_seconds)
    return worst, report
