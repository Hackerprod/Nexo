#!/usr/bin/env python3
"""Reejecuta los supervivientes sin clasificación en la copia desechable."""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument("--unit-prefix")
    choice.add_argument("--integration-test")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--include-covered", action="store_true",
                        help="comprueba toda la población vigente antes de reclasificar")
    parser.add_argument("--mutant-timeout", type=int, default=180)
    parser.add_argument("--reuse-tree", type=Path,
                        help="copia desechable con target propio; sin ella se crea una nueva")
    args = parser.parse_args()
    population = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    rows = {row["id"]: row for row in
            map(json.loads, (DIRECTORY / "results.jsonl").read_text(encoding="utf-8").splitlines())}
    covered = set(json.loads((DIRECTORY / "equivalence.json").read_text(encoding="utf-8")))
    covered |= set(json.loads((DIRECTORY / "i10a.json").read_text(encoding="utf-8")))
    name = args.unit_prefix if args.unit_prefix is not None else args.integration_test
    ids = [item["id"] for item in population
           if rows[item["id"]]["result"].get("release") == "superviviente"
           and (args.include_covered or item["id"] not in covered)
           and (args.force or not any(entry["test"] == name and entry["profile"] == "release"
                                      for entry in rows[item["id"]].get("rechecks", [])))]
    assert args.offset >= 0
    ids = ids[args.offset:]
    if args.limit is not None:
        ids = ids[:args.limit]
    print(f"Supervivientes sin clasificar: {len(ids)}", flush=True)
    if not ids:
        return
    (ROOT / ".scratch").mkdir(exist_ok=True)
    temporary = (contextlib.nullcontext(args.reuse_tree) if args.reuse_tree is not None
                 else contextlib.nullcontext(Path(tempfile.mkdtemp(prefix="i07c-recheck-", dir=ROOT / ".scratch"))))
    with temporary as directory:
        copy = Path(directory)
        if args.reuse_tree is None:
            tree = copy / "tree"
            tree.mkdir()
            for folder in ("src", "tests", "corpus", "docs", "scripts"):
                shutil.copytree(ROOT / folder, tree / folder,
                                ignore=shutil.ignore_patterns("target", "__pycache__"))
            for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
                shutil.copy2(ROOT / name, tree / name)
            (copy / "target").mkdir()
        command = [sys.executable, str(ROOT / "scripts/i07c_mutaciones.py"),
                   "--base", "b506d7d", "--ids", ",".join(ids),
                   "--reuse-tree", str(copy), "--profile", "release",
                   "--mutant-timeout", str(args.mutant_timeout)]
        if args.unit_prefix is not None:
            command += ["--unit-prefix", args.unit_prefix]
        else:
            command += ["--integration-test", args.integration_test]
        env = os.environ.copy()
        env["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "0"
        env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "256"
        subprocess.run(command, cwd=ROOT, env=env, check=True)


if __name__ == "__main__":
    main()
