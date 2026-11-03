#!/usr/bin/env python3
"""Compare each production survivor with fixed diagnostic and runtime projections."""

import argparse
import bisect
import concurrent.futures
import contextlib
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

from i07c_normative_corpus import GENERATED_COUNT, TARGETED, TEMPLATES

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
MUTATIONS = ROOT / "docs/validation/i07c-mutations"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def execute(command, cwd, env=None, timeout=300):
    return subprocess.run(command, cwd=cwd, env=env, capture_output=True,
                          timeout=timeout, check=False)


def family_boundaries() -> tuple[list[int], list[str]]:
    expanded = json.loads((ROOT / "scripts/i07c_normative_families.json").read_text(encoding="utf-8"))
    sizes = [(name, 3_000) for name, _ in TEMPLATES]
    sizes += [(name, 4_000) for name, _ in TARGETED]
    sizes += [(item["family"], 4_500) for item in expanded]
    sizes.append(("generated_mix", GENERATED_COUNT))
    endpoints, names = [], []
    count = 0
    for name, size in sizes:
        count += size
        endpoints.append(count)
        names.append(name)
    assert count == 124_000 + 4_500 * len(expanded) + GENERATED_COUNT
    return endpoints, names


def compare(base: Path, mutant: Path, corpus: Path,
            boundaries: tuple[list[int], list[str]] | None) -> tuple[int, list[dict], dict[str, int]]:
    differences = []
    total = 0
    changed_families: dict[str, int] = {}
    with base.open(encoding="utf-8") as before, mutant.open(encoding="utf-8") as after, corpus.open(encoding="utf-8") as programs:
        for number, (left, right, source) in enumerate(zip(before, after, programs, strict=True), 1):
            if left != right:
                total += 1
                if boundaries is not None:
                    endpoints, names = boundaries
                    family = names[bisect.bisect_left(endpoints, number)]
                    changed_families[family] = changed_families.get(family, 0) + 1
                if len(differences) < 5:
                    differences.append({"number": number, "source": source.rstrip("\n"),
                                        "base": left.rstrip("\n"), "mutant": right.rstrip("\n")})
    return total, differences, changed_families


def measure(item_id: str, executable: Path, pending: list[tuple[str, Path, Path]],
            tree: Path, copy: Path, env: dict[str, str],
            boundaries: tuple[list[int], list[str]] | None) -> list[dict]:
    rows = []
    try:
        for mode, sources, expected in pending:
            projection = copy / f"projection-{item_id}-{mode}.txt"
            try:
                run = execute([str(executable), mode, str(sources), str(projection)],
                              tree, env, timeout=1800)
                if run.returncode:
                    row = {"id": item_id, "mode": mode, "status": "run_failed",
                           "differences": None, "first": [],
                           "stderr": run.stderr.decode("utf-8", "replace")[-300:]}
                else:
                    count, first, changed_families = compare(
                        expected, projection, sources,
                        boundaries if mode in {"projection", "projection_full"} else None)
                    row = {"id": item_id, "mode": mode,
                           "status": "raw_difference" if count else "same",
                           "differences": count, "first": first,
                           "changed_families": changed_families}
            except subprocess.TimeoutExpired:
                row = {"id": item_id, "mode": mode, "status": "timeout",
                       "differences": None, "first": []}
            finally:
                projection.unlink(missing_ok=True)
            rows.append(row)
    finally:
        executable.unlink(missing_ok=True)
    return rows


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--copy", type=Path, required=True, help="disposable source tree parent")
    parser.add_argument("--target", type=Path, help="existing target for an otherwise idle worker")
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--runtime-corpus", type=Path)
    parser.add_argument("--runtime-baseline", type=Path)
    parser.add_argument("--runtime-output", type=Path)
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--ids", help="comma-separated subset")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--mode", choices=("projection", "projection_full", "projection_runtime"),
                        default="projection")
    parser.add_argument("--min-programs", type=int, default=400_000)
    parser.add_argument("--parallel-runs", type=int, default=1,
                        help="ejecutables mutados simultáneos por target")
    args = parser.parse_args()
    copy = args.copy.resolve()
    tree = copy / "tree"
    target = args.target.resolve() if args.target else copy / "target"
    assert ROOT in copy.parents and copy != ROOT and tree.is_dir() and target.is_dir()
    assert ROOT in target.parents and 1 <= args.parallel_runs <= 16
    corpus = args.corpus.resolve(strict=True)
    baseline = args.baseline.resolve(strict=True)
    assert len(corpus.read_text(encoding="utf-8").splitlines()) >= args.min_programs
    runtime_enabled = any(value is not None for value in
                          (args.runtime_corpus, args.runtime_baseline, args.runtime_output))
    assert not runtime_enabled or all(value is not None for value in
                                      (args.runtime_corpus, args.runtime_baseline, args.runtime_output))
    if runtime_enabled:
        assert args.mode in {"projection", "projection_full"}
        runtime_corpus = args.runtime_corpus.resolve(strict=True)
        runtime_baseline = args.runtime_baseline.resolve(strict=True)
        assert len(runtime_corpus.read_text(encoding="utf-8").splitlines()) == len(
            runtime_baseline.read_text(encoding="utf-8").splitlines())
    population = json.loads((MUTATIONS / "population.json").read_text(encoding="utf-8"))["mutations"]
    rows = {row["id"]: row for row in map(json.loads, args.results.read_text(encoding="utf-8").splitlines())}
    selected = [item for item in population if item["id"] in rows
                and rows[item["id"]]["result"]["release"] == "superviviente"]
    if args.ids:
        ids = set(args.ids.split(","))
        selected = [item for item in selected if item["id"] in ids]
        assert len(selected) == len(ids)
    if args.limit:
        selected = selected[:args.limit]
    modes = [(args.mode, corpus, baseline, args.output)]
    if runtime_enabled:
        modes.append(("projection_runtime", runtime_corpus, runtime_baseline,
                      args.runtime_output))
    previous = {}
    for mode, _, _, path in modes:
        previous[mode] = {row["id"]: row for row in map(json.loads,
            path.read_text(encoding="utf-8").splitlines())} if path.exists() else {}
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    env["CARGO_INCREMENTAL"] = "1"
    suffix = ".exe" if os.name == "nt" else ""
    exe = target / "release" / f"i07c-campaign-harness{suffix}"
    boundaries = family_boundaries() if args.mode in {"projection", "projection_full"} else None
    with contextlib.ExitStack() as stack:
        outputs = {}
        for mode, _, _, path in modes:
            path.parent.mkdir(parents=True, exist_ok=True)
            outputs[mode] = stack.enter_context(path.open("a", encoding="utf-8", newline="\n"))
        def record(rows: list[dict]) -> None:
            for row in rows:
                output = outputs[row["mode"]]
                output.write(json.dumps(row, ensure_ascii=False) + "\n")
                output.flush()
                print(f"{row['id']} {row['mode']} {row['status']} {row['differences']}", flush=True)

        with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel_runs) as executor:
            running = set()
            for item in selected:
                pending = [(mode, sources, expected) for mode, sources, expected, _ in modes
                           if item["id"] not in previous[mode]]
                if not pending:
                    continue
                source_path = tree / item["file"]
                original = source_path.read_bytes()
                assert original == (ROOT / item["file"]).read_bytes(), item["id"]
                source = original.decode("utf-8")
                assert source.count(item["literal"]) == 1, item["id"]
                mutated = source.replace(item["literal"], item["replacement"], 1)
                source_path.write_text(mutated, encoding="utf-8", newline="\n")
                try:
                    built = execute(["cargo", "build", "--locked", "--offline", "--release",
                                     "--manifest-path", str(tree / "scripts/i07c_harness/Cargo.toml")], tree, env)
                    if built.returncode:
                        built = execute(["cargo", "build", "--locked", "--offline", "--release",
                                         "--manifest-path", str(tree / "scripts/i07c_harness/Cargo.toml")], tree, env)
                    if built.returncode:
                        record([{"id": item["id"], "mode": mode, "status": "compile_failed",
                                 "differences": None, "first": [],
                                 "build_error_tail": built.stderr.decode("utf-8", "replace")[-1200:]}
                                for mode, _, _ in pending])
                    else:
                        snapshot = copy / f"mutant-{item['id']}{suffix}"
                        shutil.copy2(exe, snapshot)
                        running.add(executor.submit(measure, item["id"], snapshot, pending,
                                                    tree, copy, env, boundaries))
                except subprocess.TimeoutExpired:
                    record([{"id": item["id"], "mode": mode, "status": "timeout",
                             "differences": None, "first": []} for mode, _, _ in pending])
                finally:
                    source_path.write_bytes(original)
                    assert digest(source_path.read_bytes()) == digest(original), item["id"]
                    assert digest((ROOT / item["file"]).read_bytes()) == digest(original), item["id"]
                if len(running) >= args.parallel_runs:
                    done, running = concurrent.futures.wait(
                        running, return_when=concurrent.futures.FIRST_COMPLETED)
                    for future in done:
                        record(future.result())
            for future in concurrent.futures.as_completed(running):
                record(future.result())


if __name__ == "__main__":
    main()
