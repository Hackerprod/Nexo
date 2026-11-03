#!/usr/bin/env python3
"""Reduce un testigo diferencial usando dos binarios `nexo check` ya compilados."""

from __future__ import annotations

import argparse
import math
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TOKENS = re.compile(r"\s+|[A-Za-z_][A-Za-z_0-9]*|[0-9]+|==|!=|<=|>=|&&|\|\||->|.", re.DOTALL)


def removable_chunks(source: str) -> list[tuple[int, int]]:
    chunks: list[tuple[int, int]] = []
    for name in ("FI", "FA", "G"):
        match = re.search(rf"fn {name}\([^{{}}]*\)\s*(?:->\w+)?\s*\{{[^{{}}]*\}}\s*", source)
        if match:
            chunks.append(match.span())
    main = re.search(r"fn Main\(\)\s*\{", source)
    if not main:
        return chunks
    depth = 1
    start = main.end()
    for index in range(start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                break
            if depth == 1:
                following = source[index + 1:].lstrip()
                if not following.startswith("else"):
                    chunks.append((start, index + 1))
                    start = index + 1
        elif char == ";" and depth == 1:
            chunks.append((start, index + 1))
            start = index + 1
    return [span for span in chunks if source[slice(*span)].strip()]


def observe(exe: Path, source: str, probe: Path) -> tuple[int, bytes, bytes]:
    probe.write_text(source, encoding="utf-8", newline="\n")
    done = subprocess.run([str(exe), "check", str(probe)], cwd=ROOT,
                          capture_output=True, timeout=10, check=False)
    return done.returncode, done.stdout, done.stderr


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-exe", type=Path, required=True)
    parser.add_argument("--mutant-exe", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-probes", type=int, default=3000)
    args = parser.parse_args()
    base = args.base_exe.resolve(strict=True)
    mutant = args.mutant_exe.resolve(strict=True)
    source = args.source.read_text(encoding="utf-8")
    probe = ROOT / ".scratch" / f"i07c-reduce-{os.getpid()}.n"
    probe.parent.mkdir(exist_ok=True)
    count = 0

    def differs(candidate: str) -> bool:
        nonlocal count
        count += 1
        try:
            a = observe(base, candidate, probe)
            b = observe(mutant, candidate, probe)
            return a != b
        except subprocess.TimeoutExpired:
            return False

    assert differs(source), "el testigo no distingue los dos binarios"
    changed = True
    while changed and count < args.max_probes:
        changed = False
        for start, end in removable_chunks(source):
            candidate = source[:start] + source[end:]
            if differs(candidate):
                source = candidate
                changed = True
                break
            if count >= args.max_probes:
                break
    tokens = TOKENS.findall(source)
    granularity = 2
    while len(tokens) > 1 and count < args.max_probes:
        step = math.ceil(len(tokens) / granularity)
        reduced = False
        for start in range(0, len(tokens), step):
            candidate = tokens[:start] + tokens[start + step:]
            if candidate and differs("".join(candidate)):
                tokens = candidate
                granularity = max(2, granularity - 1)
                reduced = True
                break
            if count >= args.max_probes:
                break
        if not reduced:
            if granularity >= len(tokens):
                break
            granularity = min(len(tokens), granularity * 2)
    output = "".join(tokens)
    assert differs(output)
    args.output.write_text(output, encoding="utf-8", newline="\n")
    probe.unlink(missing_ok=True)
    print(f"probes={count} tokens={len(tokens)} bytes={len(output.encode())} output={args.output}")


if __name__ == "__main__":
    main()
