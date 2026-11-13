#!/usr/bin/env python3
"""Compare CLI bytes with HEAD, including unguarded, nonnested matches."""

import io
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
RUST_STRING = re.compile(r'r#"(.*?)"#|"((?:\\.|[^"\\])*)"', re.S)


def build_head(directory: Path) -> dict[str, Path]:
    archive = subprocess.run(["git", "archive", "HEAD"], cwd=ROOT, capture_output=True, check=True).stdout
    source = directory / "source"
    source.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            target = (source / member.name).resolve()
            if source.resolve() not in target.parents and target != source.resolve():
                raise ValueError("archive path escapes source tree")
        tar.extractall(source, filter="data")
    binaries = {}
    for mode in ("debug", "release"):
        command = ["cargo", "build", "--offline", "--locked"]
        if mode == "release":
            command.append("--release")
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(source / "target")
        subprocess.run(command, cwd=source, env=env, check=True)
        binaries[mode] = source / "target" / mode / ("nexo.exe" if os.name == "nt" else "nexo")
    return binaries


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
    previous = [row for row in rows if not re.search(r"\b(?:Option|Some|None|match)\b", row[1])]
    match_sources = [
        (f"match:{name}", source, None)
        for name, source in test_sources()
        if name.replace("\\", "/").startswith((
            "tests/j03_match.rs:", "tests/j04_match_ownership.rs:",
            "tests/j04_match_semantics.rs:",
        ))
        and len(re.findall(r"\bmatch\b", source)) == 1
        and not re.search(r"\bif\b", source)
    ]
    return previous + match_sources


def invoke(binary: Path, args: list[str]) -> tuple[int, bytes, bytes]:
    result = subprocess.run([str(binary), *args], cwd=ROOT, capture_output=True, timeout=30, check=False)
    return result.returncode, result.stdout, result.stderr


def compare() -> None:
    with tempfile.TemporaryDirectory(prefix="nexo-union-diff-", dir="D:/") as temporary:
        directory = Path(temporary)
        head = build_head(directory)
        suffix = ".exe" if os.name == "nt" else ""
        target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
        current = {mode: target / mode / f"nexo{suffix}" for mode in ("debug", "release")}
        for mode in current:
            if not current[mode].is_file():
                command = ["cargo", "build", "--offline", "--locked"]
                if mode == "release":
                    command.append("--release")
                subprocess.run(command, cwd=ROOT, check=True)
        sources = directory / "sources"
        sources.mkdir()
        rows = all_sources()
        differences = []
        commands = 0
        for index, (name, source, env) in enumerate(rows):
            if index and index % 100 == 0:
                print(f"compared_sources={index}/{len(rows)} differences={len(differences)}", flush=True)
            path = sources / f"{index:05}.n"
            path.write_text(source, encoding="utf-8", newline="")
            for mode in ("debug", "release"):
                for command in ("check", "run"):
                    args = [command, str(path), "--format", "json"]
                    if name.startswith("match:"):
                        args.extend(["--profile", "f0-0003-b-v1"])
                    if command == "run" and env:
                        args.extend(["--mock-env", str(env)])
                    before = invoke(head[mode], args)
                    after = invoke(current[mode], args)
                    commands += 1
                    if before != after:
                        differences.append((name, mode, command, before[0], after[0],
                                            before[1][:300].decode("utf-8", "replace"),
                                            after[1][:300].decode("utf-8", "replace")))
    print(json.dumps({"sources": len(rows), "commands": commands, "differences": len(differences),
                      "first": differences[:10]}, ensure_ascii=False))
    if differences:
        raise SystemExit(1)


if __name__ == "__main__":
    compare()
