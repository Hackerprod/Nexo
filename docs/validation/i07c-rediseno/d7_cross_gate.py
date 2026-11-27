#!/usr/bin/env python3
"""Run the immutable full cross gate and its SHA-frozen D7 causal complement."""

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--main-exe", type=Path, required=True)
    p.add_argument("--candidate-exe", type=Path, required=True)
    p.add_argument("--case", type=Path, required=True)
    p.add_argument("--commit", choices=[f"C{i}" for i in range(1, 7)], required=True)
    p.add_argument("--previous", type=Path)
    p.add_argument("--cross-report", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    common = ["--main-exe", str(a.main_exe), "--candidate-exe", str(a.candidate_exe),
              "--case", str(a.case), "--commit", a.commit]
    cross = [sys.executable, "-B", str(HERE / "candidate_gate.py"), *common,
             "--output", str(a.cross_report)]
    if a.previous:
        cross.extend(["--previous", str(a.previous)])
    run = subprocess.run(cross, check=False)
    if run.returncode not in (0, 1) or not a.cross_report.exists():
        raise SystemExit("frozen cross gate did not produce a complete report")
    report = json.loads(a.cross_report.read_text(encoding="utf-8"))
    if not report.get("complete"):
        raise SystemExit("incomplete cross gate cannot support a verdict")
    complement = [sys.executable, "-B", str(HERE / "d7_family_check.py"), *common,
                  "--cross-report", str(a.cross_report), "--output", str(a.output)]
    raise SystemExit(subprocess.run(complement, check=False).returncode)


if __name__ == "__main__":
    main()
