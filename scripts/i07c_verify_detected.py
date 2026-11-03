#!/usr/bin/env python3
"""Reejecuta cada mutante detectado con su prueba citada en una copia desechable."""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "docs/validation/i07c-mutations"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command: list[str], tree: Path, env: dict, timeout: int = 120):
    try:
        return subprocess.run(command, cwd=tree, env=env, capture_output=True,
                              timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        return None


def command_for(witness: str) -> tuple[str, list[str]]:
    if witness.startswith("i07c_normative_survivors: no termina"):
        name = "deferred_projection_and_constructor_programs_finish_with_causal_results"
        return name, ["--test", "i07c_normative_survivors", name]
    if ": " in witness:
        suite, suffix = witness.split(": ", 1)
        test = suffix.split(": ", 1)[0].split(",", 1)[0].strip()
        source = ROOT / "tests" / f"{suite}.rs"
        if source.is_file():
            text = source.read_text(encoding="utf-8")
            if (re.search(r"\bfn\s+" + re.escape(test) + r"\s*\(", text)
                    or re.search(r"\bmutant!\s*\(\s*" + re.escape(test) + r"\s*,", text)):
                return test, ["--test", suite, test]
    if ": " in witness and "," in witness.split(": ", 1)[0]:
        suites, suffix = witness.split(": ", 1)
        test = suffix.split(": ", 1)[0].split(",", 1)[0].strip()
        matches = [suite for suite in suites.split(",")
                   if (ROOT / "tests" / f"{suite}.rs").exists()
                   and re.search(r"\b" + re.escape(test) + r"\b",
                                 (ROOT / "tests" / f"{suite}.rs").read_text(encoding="utf-8"))]
        if len(matches) == 1:
            return test, ["--test", matches[0], test]
    name = witness.split(": ", 1)[0]
    if name in {"cargo test", "check::", "i07c_tests", "quota_tests"}:
        raise ValueError(f"no hay prueba exacta citada: {witness}")
    if "::" not in name:
        matches = [path.stem for path in (ROOT / "tests").glob("*.rs")
                   if (re.search(r"\bfn\s+" + re.escape(name) + r"\s*\(",
                                 path.read_text(encoding="utf-8"))
                       or (path.stem == "i07c_deferred_mutation_cases" and
                           re.search(r"\bmutant!\s*\(\s*" + re.escape(name) + r"\s*,",
                                     path.read_text(encoding="utf-8"))))]
        if len(matches) == 1:
            return name, ["--test", matches[0], name]
    return name, ["--lib", name]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True)
    parser.add_argument("--target", type=Path, help="target compartido solo con procesos ya inactivos")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--results", type=Path, default=DIRECTORY / "results.jsonl",
                        help="resultados completos o un fragmento terminado de la población")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--ids", help="IDs concretos separados por coma para una nueva adjudicación")
    parser.add_argument("--test-override", help="test ejecutable para adjudicar una cita ausente")
    args = parser.parse_args()
    copy = args.copy.resolve()
    tree = copy / "tree"
    target = args.target.resolve() if args.target else copy / "target"
    assert ROOT in copy.parents and copy != ROOT and target.is_dir()
    if not tree.exists():
        tree.mkdir(parents=True)
        for name in ("src", "tests", "corpus", "docs", "scripts"):
            shutil.copytree(ROOT / name, tree / name,
                            ignore=shutil.ignore_patterns("target", "__pycache__", ".scratch"))
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
            shutil.copy2(ROOT / name, tree / name)
    assert tree.is_dir()
    entries = json.loads((DIRECTORY / "population.json").read_text(encoding="utf-8"))["mutations"]
    results = {row["id"]: row for row in map(json.loads,
        args.results.read_text(encoding="utf-8").splitlines())}
    selected = [item for item in entries if item["id"] in results and
                (args.test_override or results[item["id"]]["result"].get("release") == "detectada")]
    assert not args.test_override or args.ids
    if args.ids:
        wanted = set(args.ids.split(","))
        assert all(re.fullmatch(r"M\d{5}", value) for value in wanted)
        selected = [item for item in selected if item["id"] in wanted]
        assert len(selected) == len(wanted)
    if args.limit:
        selected = selected[:args.limit]
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    env["CARGO_INCREMENTAL"] = "1"
    env["RUST_TEST_THREADS"] = "4"
    env["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "0"
    env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "256"
    prior = {}
    if args.output.exists():
        prior = {row["id"]: row for row in map(json.loads,
            args.output.read_text(encoding="utf-8").splitlines())}
    # Test every distinct cited baseline before the first substitution. This
    # keeps the exact-test check while avoiding a clean rebuild between mutants.
    baselines = {}
    for item in selected:
        if item["id"] in prior:
            continue
        witness = args.test_override or results[item["id"]].get("witness", {}).get("release", "")
        try:
            name, selection = command_for(witness)
        except ValueError:
            continue
        key = (name, tuple(selection))
        if key in baselines:
            continue
        command = ["cargo", "test", "--locked", "--offline", "--release", *selection]
        exact = "--test" in selection or "::" in name
        test_args = ["--", *(["--exact"] if exact else []),
                     *(["--ignored"] if "i07c_mutant_projection_scale" in selection else []),
                     "--nocapture"]
        start = time.monotonic()
        base = run([*command, *test_args], tree, env)
        elapsed = time.monotonic() - start
        if (elapsed >= 10 and base is not None and base.returncode == 0
                and b"test result: ok. 1 passed; 0 failed;" in base.stdout):
            start = time.monotonic()
            base = run([*command, *test_args], tree, env)
            elapsed = time.monotonic() - start
        baselines[key] = (base is not None and base.returncode == 0
                          and b"test result: ok. 1 passed; 0 failed;" in base.stdout,
                          round(elapsed, 3))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("a", encoding="utf-8", newline="\n") as output:
        for item in selected:
            if item["id"] in prior:
                continue
            prior_witness = results[item["id"]].get("witness", {}).get("release", "")
            witness = args.test_override or prior_witness
            try:
                name, selection = command_for(witness)
            except ValueError as error:
                row = {"id": item["id"], "status": "uncited", "detail": str(error)}
                output.write(json.dumps(row, ensure_ascii=False) + "\n")
                output.flush()
                continue
            source = tree / item["file"]
            original = source.read_bytes()
            assert original == (ROOT / item["file"]).read_bytes(), item["id"]
            assert original.decode("utf-8").count(item["literal"]) == 1
            command = ["cargo", "test", "--locked", "--offline", "--release", *selection]
            exact = "--test" in selection or "::" in name
            test_args = ["--", *(["--exact"] if exact else []),
                         *(["--ignored"] if "i07c_mutant_projection_scale" in selection else []),
                         "--nocapture"]
            baseline_ok, baseline_seconds = baselines[(name, tuple(selection))]
            if not baseline_ok:
                row = {"id": item["id"], "test": name, "status": "uncited",
                       "witness": witness, "baseline_seconds": round(baseline_seconds, 3),
                       "source_sha256": sha(source)}
                output.write(json.dumps(row, ensure_ascii=False) + "\n")
                output.flush()
                print(item["id"], "uncited", name, flush=True)
                continue
            source.write_text(original.decode("utf-8").replace(item["literal"], item["replacement"], 1),
                              encoding="utf-8", newline="\n")
            try:
                built = run([*command, "--no-run"], tree, env, 180)
                observed = (run([*command, *test_args], tree, env, 30)
                            if built is not None and built.returncode == 0 else None)
            finally:
                source.write_bytes(original)
            assert sha(source) == hashlib.sha256(original).hexdigest()
            if built is None:
                status = "inconclusive_build_timeout"
            elif built.returncode:
                status = "no_compile"
            elif observed is None:
                status = "detected_timeout" if baseline_seconds < 10 else "inconclusive_timeout"
            else:
                text = observed.stdout + observed.stderr
                status = ("detected" if observed.returncode and
                          b"test result: FAILED. 0 passed; 1 failed;" in text else
                          "not_killed" if observed.returncode == 0 and
                          b"test result: ok. 1 passed; 0 failed;" in text else
                          "inconclusive")
            row = {"id": item["id"], "test": name, "status": status,
                   "witness": witness, "baseline_seconds": round(baseline_seconds, 3),
                   "source_sha256": sha(source)}
            if args.test_override:
                row["prior_witness"] = prior_witness
            output.write(json.dumps(row, ensure_ascii=False) + "\n")
            output.flush()
            print(item["id"], status, name, flush=True)


if __name__ == "__main__":
    main()
