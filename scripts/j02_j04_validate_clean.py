#!/usr/bin/env python3
"""Run the repository validator from a clean copy without transient files."""
from __future__ import annotations

from pathlib import Path
import os
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="nexo-j-validate-", dir="D:/") as temporary:
        copy = Path(temporary) / "source"
        shutil.copytree(ROOT, copy, ignore=shutil.ignore_patterns(
            ".git", "target", ".scratch", "__pycache__", ".i11a-mutation-copy"))
        if (copy / "target").exists() or (copy / ".scratch").exists():
            raise RuntimeError("transient directory entered the clean copy")
        with (Path(temporary) / "validate.log").open("wb") as log:
            env = dict(os.environ, CARGO_TARGET_DIR=str(copy / "target"))
            done = subprocess.run(["sh", "scripts/validate.sh"], cwd=copy, env=env,
                                  stdout=log, stderr=subprocess.STDOUT, check=False)
        output = (Path(temporary) / "validate.log").read_text(encoding="utf-8", errors="replace")
        print(f"clean_validate_exit={done.returncode}")
        print(output[-4000:])
        return done.returncode


if __name__ == "__main__":
    raise SystemExit(main())
