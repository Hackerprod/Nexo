#!/usr/bin/env python3
"""Ejecuta y registra las campañas deterministas de I07c."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HARNESS = ROOT / "scripts/i07c_harness"
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / ".scratch/i07c-campaign-target"))
LOGS = ROOT / ".scratch/i07c-campaign-logs"
RESULTS = ROOT / "docs/validation/i07c-campaign-results"
REGISTRY = ROOT / "docs/validation/i07c-campaign-results.json"
BASE = "b506d7d"
DEPTHS = "4,16,64,100,126,127,128,129,150,300,1000,4000,12000"

SPECS = (
    ("groups_release", "release", "grp", 150_000, 7_310_101),
    ("groups_debug", "debug", "grp", 20_000, 7_310_102),
    ("model_release", "release", "model", 250_000, 7_310_103),
    ("model_newseed_release", "release", "model", 120_000, 7_310_104),
    ("deferred_release", "release", "defer", 219_116, 7_310_105),
    ("oracle_release", "release", "oracle", 295_606, 7_310_106),
    ("fuzz_release", "release", "fuzz", 400_000, 7_310_107),
    ("fuzz_debug", "debug", "fuzz", 100_000, 7_310_108),
    ("types_release", "release", "types", 20_000, 7_310_109),
    *((f"stacks_{profile}_{kib}", profile, "deep", kib, 0)
      for profile in ("release", "debug") for kib in (128, 256, 512)),
)
ZERO = {
    "grp": ("sig_diffs", "cardinality_diffs", "group_spans", "raw_spans",
            "strict_dup_programs", "same_site_programs", "bugs"),
    "model": ("accept_diff", "count_diff", "field_diff", "dups", "bugs"),
    "defer": ("acceptance_diff", "q_lost", "strict_dup_programs",
              "same_site_programs", "bugs"),
    "oracle": ("oracle_disagreements", "order_dependent_programs",
               "check_run_incoherent", "strict_dups", "bugs"),
    "fuzz": ("bugs_panic_invalidir_internal", "check_run_incoherent",
             "strict_dup_programs", "same_site_programs"),
    "types": ("field_diff", "duplicates", "bugs"),
}


def run_bytes(command: list[str], cwd: Path, env: dict[str, str]) -> bytes:
    done = subprocess.run(command, cwd=cwd, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=False)
    if done.returncode:
        raise RuntimeError(f"{command!r}: exit {done.returncode}\n"
                           + done.stdout[-3000:].decode("utf-8", "replace")
                           + done.stderr[-3000:].decode("utf-8", "replace"))
    return done.stdout


def binary(profile: str) -> Path:
    return TARGET / profile / ("i07c-campaign-harness.exe" if os.name == "nt"
                               else "i07c-campaign-harness")


def build(profile: str) -> None:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TARGET)
    args = ["cargo", "build", "--offline", "--manifest-path", str(HARNESS / "Cargo.toml")]
    if profile == "release":
        args.append("--release")
    subprocess.run(args, cwd=ROOT, env=env, check=True)


def record(name: str, mode: str, stdout: bytes, programs: int, checks: int,
           summary: dict, seed: int | list[int], threads: int) -> dict:
    LOGS.mkdir(parents=True, exist_ok=True)
    RESULTS.mkdir(parents=True, exist_ok=True)
    (LOGS / f"{name}.log").write_bytes(stdout)
    result = {"name": name, "mode": mode.upper(), "programs": programs,
              "checks": checks, "summary": summary,
              "seed": seed, "threads": threads,
              "informative": name in {"quota_reduced_release", "deferred_release"}}
    path = RESULTS / f"{name}.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8", newline="\n")
    return {"name": name, "result_file": path.relative_to(ROOT).as_posix(),
            "programs": programs, "checks": checks,
            "informative": name in {"quota_reduced_release", "deferred_release"}}


def ordinary(spec: tuple, threads: int) -> dict:
    name, profile, mode, size, seed = spec
    exe = binary(profile)
    if mode == "deep":
        args = [str(exe), mode, str(size), DEPTHS]
    else:
        args = [str(exe), mode, str(size), str(seed), str(threads)]
    stdout = run_bytes(args, ROOT, os.environ.copy())
    LOGS.mkdir(parents=True, exist_ok=True)
    (LOGS / f"{name}.log").write_bytes(stdout)
    lines = stdout.decode("utf-8").splitlines()
    if mode == "deep":
        cases = [line for line in lines if line.startswith("CASE ")]
        results = [line for line in lines if line.startswith("RESULT ")]
        assert cases and len(cases) == len(results), name
        assert all("coherent=true dup=false" in line and "BUG" not in line
                   for line in results), name
        programs = len(cases)
        checks = 2 * programs
        summary = {"cases": programs, "coherent": programs,
                   "duplicates": 0, "bugs": 0}
    else:
        prefix = mode.upper()
        matches = [line for line in lines if line.startswith(prefix)
                   and re.search(r"(?:pairs|programs|seeds)=", line)]
        assert len(matches) == 1, (name, matches)
        summary = {key: int(value) for key, value in
                   re.findall(r"([a-z_]+)=([0-9]+)", matches[0])}
        assert all(summary.get(field) == 0 for field in ZERO[mode]), (name, summary)
        programs = summary.get("programs", summary.get("compile_runs"))
        assert programs is not None, name
        checks = programs * 2 if mode in {"fuzz", "types"} else programs
    return record(name, mode, stdout, programs, checks, summary, seed, threads)


def quota() -> dict:
    seeds = list(range(101, 113))
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TARGET)
    env["I07C_QUOTA_SEEDS"] = ",".join(map(str, seeds))
    env["I07C_QUOTA_PER"] = "400"
    command = ["cargo", "test", "--offline", "--release", "--lib",
               "reduced_quota_property", "--", "--ignored", "--nocapture"]
    done = subprocess.run(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, check=False)
    if done.returncode:
        raise RuntimeError(done.stdout[-4000:].decode("utf-8", "replace"))
    line = next(line for line in done.stdout.decode("utf-8", "replace").splitlines()
                if "TOTAL programs " in line)
    numbers = {key: int(value) for key, value in
               re.findall(r"([a-z_]+) ([0-9]+)", line)}
    assert numbers["programs"] == 4_800 and numbers["checks"] == 268_800
    assert numbers["skipped"] == numbers["violations"] == 0
    return record("quota_reduced_release", "quota_property", (line + "\n").encode(),
                  numbers["programs"], numbers["checks"], numbers, seeds, 1)


def base_tree(target: Path) -> None:
    archive = subprocess.run(["git", "archive", BASE], cwd=ROOT,
                             stdout=subprocess.PIPE, check=True).stdout
    target.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            resolved = (target / member.name).resolve()
            assert target.resolve() == resolved or target.resolve() in resolved.parents
        tar.extractall(target, filter="data")
    shutil.copytree(HARNESS, target / "scripts/i07c_harness",
                    ignore=shutil.ignore_patterns("target", "__pycache__"))
    shutil.copy2(ROOT / "scripts/i07c_common.rs", target / "scripts/i07c_common.rs")


def acceptance() -> dict:
    cases = json.loads((RESULTS / "head_acceptance.json").read_text(encoding="utf-8"))["cases"]
    (ROOT / ".scratch").mkdir(exist_ok=True)
    with contextlib.nullcontext(Path(tempfile.mkdtemp(prefix="i07c-base-", dir=ROOT / ".scratch"))) as temporary:
        tree = temporary / "tree"
        base_tree(tree)
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = os.environ.get(
            "I07C_BASE_TARGET", str(Path(temporary) / "target")
        )
        override = os.environ.get("I07C_BASE_EXE")
        if override:
            base_exe = Path(override).resolve(strict=True)
        else:
            subprocess.run(["cargo", "build", "--offline", "--release", "--manifest-path",
                            str(tree / "scripts/i07c_harness/Cargo.toml")],
                           cwd=tree, env=env, check=True)
            base_exe = Path(env["CARGO_TARGET_DIR"]) / "release" / binary("release").name
        rows = []
        log = []
        for case in cases:
            source = Path(temporary) / f"case-{len(rows):03}.n"
            source.write_text(case["source"], encoding="utf-8", newline="\n")
            old = run_bytes([str(base_exe), "diag", str(source)], tree, env).decode("utf-8")
            current = run_bytes([str(binary("release")), "diag", str(source)], ROOT,
                                os.environ.copy()).decode("utf-8")
            old_ok = "check: Ok" in old
            current_ok = "check: Ok" in current
            assert old_ok != current_ok, case["id"]
            rows.append({"id": case["id"], "kind": case["kind"],
                         "source": case["source"], "base_accepts": old_ok,
                         "current_accepts": current_ok})
            log.append(f"{case['id']} base={old_ok} current={current_ok}")
    summary = {"acceptance_differences": len(rows), "unexpected": 0,
               "families": sorted({row["kind"] for row in rows})}
    entry = record("head_acceptance", "replay", ("\n".join(log) + "\n").encode(),
                   len(rows), len(rows) * 2, summary, 0, 1)
    path = RESULTS / "head_acceptance.json"
    result = json.loads(path.read_text(encoding="utf-8"))
    result["cases"] = rows
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8", newline="\n")
    return entry


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", nargs="*", help="nombres de campañas; omitir para todas")
    parser.add_argument("--threads", type=int, default=4)
    args = parser.parse_args()
    names = set(args.only or [spec[0] for spec in SPECS]
                + ["head_acceptance", "quota_reduced_release"])
    known = {spec[0] for spec in SPECS} | {"head_acceptance", "quota_reduced_release"}
    assert names <= known, names - known
    profiles = {profile for name, profile, *_ in SPECS if name in names}
    if "head_acceptance" in names:
        profiles.add("release")
    for profile in sorted(profiles):
        build(profile)
    entries = []
    for spec in SPECS:
        if spec[0] in names:
            entries.append(ordinary(spec, args.threads))
            print(spec[0], entries[-1]["programs"], flush=True)
    if "quota_reduced_release" in names:
        entries.append(quota())
    if "head_acceptance" in names:
        entries.append(acceptance())
    prior = {row["name"]: row for row in json.loads(REGISTRY.read_text(encoding="utf-8"))}
    prior.update({row["name"]: row for row in entries})
    ordered = [prior[spec[0]] for spec in SPECS] + [prior["head_acceptance"],
                                                 prior["quota_reduced_release"]]
    REGISTRY.write_text(json.dumps(ordered, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"{len(entries)} campañas ejecutadas; índice {REGISTRY.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
