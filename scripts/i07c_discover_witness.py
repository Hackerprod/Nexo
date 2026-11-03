#!/usr/bin/env python3
"""Find a current suite failure for an obsolete mutation witness in a disposable tree."""

import argparse
import hashlib
import json
import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


@dataclass
class TimedOut:
    stdout: bytes
    stderr: bytes


def execute(command: list[str], tree: Path, env: dict, timeout: int):
    try:
        return subprocess.run(command, cwd=tree, env=env, capture_output=True,
                              timeout=timeout, check=False)
    except subprocess.TimeoutExpired as error:
        return TimedOut(error.stdout or b"", error.stderr or b"")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--ids", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--skip-extensive", action="store_true",
                        help="run exact library and bounded integration suites first")
    args = parser.parse_args()
    copy = args.copy.resolve()
    tree = copy / "tree"
    target = args.target.resolve()
    assert ROOT in copy.parents and tree.is_dir() and target.is_dir()
    population = {item["id"]: item for item in
        json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]}
    ids = args.ids.split(",")
    assert len(ids) == len(set(ids)) and set(ids) <= population.keys()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    env["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "0"
    env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "256"
    env["RUST_TEST_THREADS"] = "4"
    previous = {}
    if args.output.exists():
        previous = {row["id"]: row for row in map(json.loads,
            args.output.read_text(encoding="utf-8").splitlines())}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("a", encoding="utf-8", newline="\n") as output:
        for id in ids:
            if id in previous:
                continue
            item = population[id]
            path = tree / item["file"]
            original = path.read_bytes()
            assert original == (ROOT / item["file"]).read_bytes(), id
            source = original.decode("utf-8")
            assert source.count(item["literal"]) == 1
            path.write_text(source.replace(item["literal"], item["replacement"], 1),
                            encoding="utf-8", newline="\n")
            try:
                build = execute(["cargo", "test", "--locked", "--offline", "--release",
                                 "--no-run"], tree, env, 180)
                if isinstance(build, TimedOut):
                    row = {"id": id, "status": "build_timeout", "failed": [],
                           "tail": (build.stdout + build.stderr).decode("utf-8", "replace")[-500:]}
                elif build.returncode:
                    row = {"id": id, "status": "build_failed", "failed": [],
                           "tail": build.stderr.decode("utf-8", "replace")[-500:]}
                else:
                    row = None
                    extensive = {"i07c_deferred_metamorphic", "i07c_deferred_twins"}
                    suites = sorted(path.stem for path in (ROOT / "tests").glob("*.rs"))
                    suites.sort(key=lambda name: name in extensive)
                    checks = [("library", ["--lib"], 300)] + [
                        (name, ["--test", name], 600 if name in extensive else 180)
                        for name in suites if not args.skip_extensive or name not in extensive
                    ]
                    timed_out = []
                    for kind, selection, timeout in checks:
                        run = execute(["cargo", "test", "--locked", "--offline", "--release",
                                       *selection, "--", "--nocapture"], tree, env, timeout)
                        if isinstance(run, TimedOut):
                            log = (run.stdout + run.stderr).decode("utf-8", "replace")
                            failed = re.findall(r"(?m)^test ([^\r\n]+?) \.\.\. FAILED$", log)
                            if failed:
                                row = {"id": id, "status": "failed", "suite": kind,
                                       "failed": failed, "tail": log[-1200:]}
                                break
                            timed_out.append(kind)
                            continue
                        log = (run.stdout + run.stderr).decode("utf-8", "replace")
                        failed = re.findall(r"(?m)^test ([^\r\n]+?) \.\.\. FAILED$", log)
                        if failed or run.returncode:
                            row = {"id": id, "status": "failed" if failed else "inconclusive",
                                   "suite": kind, "failed": failed, "exit_code": run.returncode,
                                   "tail": log[-500:] if run.returncode and not failed else ""}
                            break
                    if row is None:
                        row = {"id": id, "status": "suite_timeout" if timed_out else
                               "no_fast_failure" if args.skip_extensive else "no_failure",
                               "failed": [], "timed_out": timed_out}
            finally:
                path.write_bytes(original)
            assert hashlib.sha256(path.read_bytes()).digest() == hashlib.sha256(original).digest()
            assert (ROOT / item["file"]).read_bytes() == original
            row["source_sha256"] = hashlib.sha256(original).hexdigest()
            output.write(json.dumps(row, ensure_ascii=False) + "\n")
            output.flush()
            print(id, row["status"], ",".join(row["failed"][:2]), flush=True)


if __name__ == "__main__":
    main()
