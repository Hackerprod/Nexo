#!/usr/bin/env python3
"""Ejecuta validate.sh en una copia limpia y guarda la medición reproducible."""

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/i07c-validate.json"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True)
    parser.add_argument("--reuse", action="store_true",
                        help="use an existing clean tree with an empty target")
    args = parser.parse_args()
    copy = args.copy.resolve()
    assert ROOT in copy.parents, copy
    tree = copy / "tree"
    target = copy / "target"
    if args.reuse:
        from i07c_mutaciones import assert_disposable_inputs
        assert tree.is_dir() and target.is_dir() and not any(target.iterdir())
        assert_disposable_inputs(tree)
    else:
        assert not copy.exists(), copy
        tree.mkdir(parents=True)
        for name in ("src", "tests", "corpus", "docs", "scripts"):
            shutil.copytree(ROOT / name, tree / name,
                            ignore=shutil.ignore_patterns("target", "__pycache__", ".scratch"))
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".gitignore", ".gitattributes"):
            source = ROOT / name
            if source.exists():
                shutil.copy2(source, tree / name)
    assert not (tree / ".scratch").exists()
    if not args.reuse:
        target.mkdir()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    done = subprocess.run(["sh", "scripts/validate.sh"], cwd=tree, env=env,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    log = copy / "validate.log"
    log.write_bytes(done.stdout)
    result = {
        "command": "sh scripts/validate.sh",
        "clean_copy_without_scratch": True,
        "fresh_target": True,
        "exit_code": done.returncode,
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    assert done.returncode == 0, log
    print(log)


if __name__ == "__main__":
    main()
