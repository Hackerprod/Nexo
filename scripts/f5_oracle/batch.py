"""Run F5 oracle seeds in resumable C: batches under the shared heavy lock."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from .runner import CAPABILITIES, manifest, seed_range

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "scripts/heavy-lock.py"


def make_batches(spec: str, width: int) -> list[list[int]]:
    if width < 1:
        raise ValueError("batch width must be positive")
    seeds = list(seed_range(spec))
    return [seeds[offset:offset + width] for offset in range(0, len(seeds), width)]


def validate_scale(phase: str, complete: str, gap_seconds: float,
                   output_dir: Path) -> tuple[str, ...]:
    active = manifest(phase, complete)
    if phase == "G6" and len(active) != len(CAPABILITIES):
        raise ValueError("G6 scale requires all L01b..L12 capabilities complete")
    if gap_seconds < 60:
        raise ValueError("heavy-lock gap must be at least 60 seconds")
    if output_dir.resolve().drive.upper() != "C:":
        raise ValueError("output-dir must be on C:")
    return active


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-debug", required=True, type=Path)
    parser.add_argument("--binary-release", required=True, type=Path)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--phase", required=True)
    parser.add_argument("--complete", required=True)
    parser.add_argument("--seeds", default="0:31")
    parser.add_argument("--valid-per-seed", type=int, default=1000)
    parser.add_argument("--invalid-per-seed", type=int, default=1000)
    parser.add_argument("--batch-seeds", type=int, default=1)
    parser.add_argument("--gap-seconds", type=float, default=75)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--session", default="f5-oracle-o4")
    parser.add_argument("--continue-on-divergence", action="store_true")
    args = parser.parse_args()
    try:
        validate_scale(args.phase, args.complete, args.gap_seconds, args.output_dir)
        batches = make_batches(args.seeds, args.batch_seeds)
    except ValueError as error:
        parser.error(str(error))
    args.output_dir.mkdir(parents=True, exist_ok=True)
    summary = []
    for offset, group in enumerate(batches):
        label = f"{group[0]:02d}-{group[-1]:02d}"
        output = args.output_dir / f"batch-{label}.jsonl"
        command = [sys.executable, "-B", str(LOCK), "run", "--session",
                   f"{args.session}-{label}", "--", sys.executable, "-B",
                   "-m", "scripts.f5_oracle.cli", "gate",
                   "--binary-debug", str(args.binary_debug.resolve()),
                   "--binary-release", str(args.binary_release.resolve()),
                   "--profile", args.profile, "--phase", args.phase,
                   "--complete", args.complete,
                   "--seeds", f"{group[0]}:{group[-1]}",
                   "--valid-per-seed", str(args.valid_per_seed),
                   "--invalid-per-seed", str(args.invalid_per_seed),
                   "--output", str(output)]
        if offset:
            command.append("--no-literals")
        started = time.monotonic()
        result = subprocess.run(command, cwd=ROOT, capture_output=True,
                                text=True, encoding="utf-8")
        print(result.stdout, end="", flush=True)
        if result.stderr:
            print(result.stderr, file=sys.stderr, end="", flush=True)
        try:
            report = next(json.loads(line) for line in reversed(result.stdout.splitlines())
                          if line.startswith("{") and "cases_executed" in line)
        except StopIteration:
            raise RuntimeError(f"batch {label} ended without a campaign report")
        report["batch"] = label
        report["exit"] = result.returncode
        report["wall_with_lock_seconds"] = round(time.monotonic() - started, 3)
        summary.append(report)
        (args.output_dir / "summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
            encoding="utf-8")
        if result.returncode and not (args.continue_on_divergence and
                                      report.get("differences", 0) > 0):
            return result.returncode
        if offset + 1 < len(batches):
            print(f"Heavy-lock gap: {args.gap_seconds:.0f}s", flush=True)
            time.sleep(args.gap_seconds)
    return 1 if any(report["differences"] for report in summary) else 0


if __name__ == "__main__":
    raise SystemExit(main())
