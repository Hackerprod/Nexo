#!/usr/bin/env python3
"""Compare the current CLI with binaries built from git archive HEAD.

Run ``--build-head`` after saving current debug/release binaries to
``.scratch/current-union-bin`` and removing the active target. Output is byte exact.
"""

import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
HEAD_BIN = ROOT / ".scratch" / "head-union-bin"
CURRENT_BIN = ROOT / ".scratch" / "current-union-bin"
SOURCES = ROOT / ".scratch" / "union-diff-sources"
RUST_STRING = re.compile(r'r#"(.*?)"#|"((?:\\.|[^"\\])*)"', re.S)


def build_head() -> None:
    archive = subprocess.run(["git", "archive", "HEAD"], cwd=ROOT, capture_output=True, check=True).stdout
    with tempfile.TemporaryDirectory(prefix="nexo-union-head-", dir=ROOT / ".scratch") as temporary:
        tree = Path(temporary)
        tar_path = tree / "head.tar"
        tar_path.write_bytes(archive)
        source = tree / "source"
        source.mkdir()
        with tarfile.open(tar_path) as tar:
            for member in tar.getmembers():
                target = (source / member.name).resolve()
                if source.resolve() not in target.parents and target != source.resolve():
                    raise ValueError("archive path escapes source tree")
            tar.extractall(source, filter="data")
        HEAD_BIN.mkdir(parents=True, exist_ok=True)
        for mode in ("debug", "release"):
            command = ["cargo", "build", "--offline", "--locked"]
            if mode == "release":
                command.append("--release")
            subprocess.run(command, cwd=source, check=True)
            binary = source / "target" / mode / ("nexo.exe" if os.name == "nt" else "nexo")
            shutil.copy2(binary, HEAD_BIN / f"head-{mode}{binary.suffix}")
            shutil.rmtree(source / "target")
        print("HEAD", subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip())


def unescape_rust(literal: str) -> str:
    # Rust fixture strings use the JSON-compatible subset of escapes here.
    try:
        return json.loads('"' + literal + '"')
    except json.JSONDecodeError:
        return literal.replace(r"\n", "\n").replace(r'\"', '"').replace(r"\\", "\\")


def test_sources() -> list[tuple[str, str]]:
    sources = []
    for path in sorted((ROOT / "tests").rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        for index, match in enumerate(RUST_STRING.finditer(text)):
            value = match.group(1) if match.group(1) is not None else unescape_rust(match.group(2))
            value = value.replace(chr(0x27e6), "").replace(chr(0x27e7), "")
            if re.search(r"\b(?:fn|gen fn)\s+\w+\s*\(", value):
                sources.append((f"{path.relative_to(ROOT)}:{index}", value))
    return sources


def f0007_examples() -> list[tuple[str, str]]:
    adr = ROOT / "docs/adr/F0-0007-inferencia-diferida-constructores-result.md"
    rows = []
    for line in adr.read_text(encoding="utf-8").splitlines():
        match = re.match(r"\| ((?:V|N)\d+) /.*?`(fn .*?)`", line)
        if match:
            rows.append((f"F0-0007:{match.group(1)}",
                         match.group(2).replace(chr(0x27e6), "").replace(chr(0x27e7), "")))
    if len(rows) != 22:
        raise ValueError(f"expected 22 F0-0007 examples, found {len(rows)}")
    return rows


def all_sources() -> list[tuple[str, str, Path | None]]:
    rows = []
    fixture = json.loads((ROOT / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))
    rows.extend((f"RD:{row['id']}", row["source"], None) for row in fixture["rows"])
    for directory in ("pass", "fail", "objetivo"):
        for path in sorted((ROOT / "corpus" / directory).glob("*.n")):
            env = path.with_suffix(".env.json")
            rows.append((str(path.relative_to(ROOT)), path.read_text(encoding="utf-8"), env if env.exists() else None))
    rows.extend((name, source, None) for name, source in test_sources())
    rows.extend((name, source, None) for name, source in f0007_examples())
    for directory in ("f0007-structural", "f0007-binding-sweep"):
        for path in sorted((ROOT / ".scratch" / directory).glob("*.n")):
            rows.append((f"{directory}:{path.name}", path.read_text(encoding="utf-8"), None))
    assignments = ["Ok(1)", "Err(2)", "Err(true)", "Ok([1])", "Err([2])"]
    contexts = ["", "let s:Result<I64,I64>=r;", "let s:Result<I64,Bool>=r;",
                "if true {r=Ok(3);}", "while false {r=Err(4);}",
                "let alias=r;Drop(alias);", "Drop(r);r=Ok(3);",
                "r=Ok(3);r=Err(4);", "if false {r=Err(true);}",
                "while false {r=Ok([3]);}", "let a=r;AssertEq(a,a);",
                "r=Err([3]);r=Ok([4]);"]
    combinations = itertools.product(assignments, assignments, contexts)
    for index, (first, second, context) in enumerate(itertools.islice(combinations, 300)):
        source = f"fn Main(){{var r={first};r={second};{context}}}"
        rows.append((f"f0007-reassignment:{index:03}", source, None))
    return rows


def invoke(binary: Path, args: list[str]) -> tuple[int, bytes, bytes]:
    result = subprocess.run([str(binary), *args], cwd=ROOT, capture_output=True, timeout=30, check=False)
    return result.returncode, result.stdout, result.stderr


def head_catalog_codes() -> list[str]:
    catalog = subprocess.check_output(
        ["git", "show", "HEAD:docs/adr/F0-CODIGOS.md"], cwd=ROOT
    ).decode("utf-8")
    return sorted(set(re.findall(r"\bNX-E\d{4}\b", catalog)))


def compare() -> None:
    suffix = ".exe" if os.name == "nt" else ""
    binaries = {
        (lineage, mode): (HEAD_BIN / f"head-{mode}{suffix}" if lineage == "head"
                          else CURRENT_BIN / f"current-{mode}{suffix}")
        for lineage in ("head", "current") for mode in ("debug", "release")
    }
    for binary in binaries.values():
        if not binary.is_file():
            raise SystemExit(f"missing binary: {binary}")
    SOURCES.mkdir(parents=True, exist_ok=True)
    rows = all_sources()
    differences = []
    commands = 0
    for index, (name, source, env) in enumerate(rows):
        if index and index % 100 == 0:
            print(f"compared_sources={index}/{len(rows)} differences={len(differences)}", flush=True)
        path = SOURCES / f"{index:05}.n"
        path.write_text(source, encoding="utf-8", newline="")
        for mode in ("debug", "release"):
            for command in ("check", "run"):
                args = [command, str(path), "--format", "json"]
                if command == "run" and env:
                    args.extend(["--mock-env", str(env)])
                before = invoke(binaries["head", mode], args)
                after = invoke(binaries["current", mode], args)
                commands += 1
                if before != after:
                    differences.append((name, mode, command, hashlib.sha256(repr(before).encode()).hexdigest(),
                                        hashlib.sha256(repr(after).encode()).hexdigest()))
    for mode in ("debug", "release"):
        for code in head_catalog_codes():
            before = invoke(binaries["head", mode], ["explain", code, "--format", "json"])
            after = invoke(binaries["current", mode], ["explain", code, "--format", "json"])
            commands += 1
            if before != after:
                differences.append((code, mode, "explain", hashlib.sha256(repr(before).encode()).hexdigest(),
                                    hashlib.sha256(repr(after).encode()).hexdigest()))
    print(json.dumps({"sources": len(rows), "commands": commands, "differences": len(differences),
                      "first": differences[:10]}, ensure_ascii=False))
    if differences:
        raise SystemExit(1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-head", action="store_true")
    args = parser.parse_args()
    build_head() if args.build_head else compare()
