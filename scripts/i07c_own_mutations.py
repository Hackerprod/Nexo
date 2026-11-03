#!/usr/bin/env python3
"""Repite las sustituciones propias en copia aislada y verifica restauración SHA-256."""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/i07c-own-mutations.json"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(command: list[str], tree: Path, env: dict) -> int:
    done = subprocess.run(command, cwd=tree, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, timeout=300, check=False)
    return done.returncode


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True)
    args = parser.parse_args()
    copy = args.copy.resolve()
    assert ROOT in copy.parents and not copy.exists()
    prior = json.loads(OUTPUT.read_text(encoding="utf-8"))
    tree = copy / "tree"
    tree.mkdir(parents=True)
    for name in ("src", "tests", "corpus", "docs", "scripts"):
        shutil.copytree(ROOT / name, tree / name,
                        ignore=shutil.ignore_patterns("target", "__pycache__", ".scratch"))
    for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        shutil.copy2(ROOT / name, tree / name)
    target = copy / "target"
    target.mkdir()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    env["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "0"
    env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "256"
    command = ["cargo", "test", "--locked", "--offline", "--release"]
    directed = command + ["--test", "i07c_normative_survivors", prior["witness"]]
    assert run(command + ["--no-run"], tree, env) == 0
    rows = []
    for item in prior["mutations"]:
        candidate = dict(item["candidate"])
        path = tree / candidate["file"]
        original = path.read_bytes()
        assert original == (ROOT / candidate["file"]).read_bytes()
        source = original.decode("utf-8")
        assert source.count(candidate["literal"]) == 1, candidate
        changed = source.replace(candidate["literal"], candidate["replacement"], 1)
        candidate["line"] = source[:source.index(candidate["literal"])].count("\n") + 1
        path.write_text(changed, encoding="utf-8", newline="\n")
        try:
            mutated = sha(path.read_bytes())
            assert run(command + ["--no-run"], tree, env) == 0
            suite_exit = run(command, tree, env)
            directed_exit = run(directed, tree, env)
        finally:
            path.write_bytes(original)
        assert sha(path.read_bytes()) == sha(original)
        row = {"candidate": candidate, "id": item["id"], "exit_code": suite_exit,
               "directed_exit_code": directed_exit,
               "normative_survivor_test_failed": directed_exit != 0,
               "mutated_sha256": mutated, "original_sha256": sha(original),
               "restored_sha256": sha(path.read_bytes())}
        rows.append(row)
        assert suite_exit != 0 and directed_exit != 0, item["id"]
    prior["mutations"] = rows
    OUTPUT.write_text(json.dumps(prior, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print(len(rows))


if __name__ == "__main__":
    main()
