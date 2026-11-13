#!/usr/bin/env python3
"""Compare J02-J04, corpus and RD across CLI commands and build modes."""
from __future__ import annotations

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
J_PROFILE = "f0-0003-b-v1"
A_PROFILE = "f0-0002-b-v2"
RUST_STRING = re.compile(r'r#"(.*?)"#|"((?:\\.|[^"\\])*)"', re.S)


def examples() -> list[tuple[str, str, str, Path | None, Path | None]]:
    rows = []
    for name in ("j02_option", "j03_match", "j04_match_ownership", "j04_match_semantics"):
        path = ROOT / "tests" / f"{name}.rs"
        for index, match in enumerate(RUST_STRING.finditer(path.read_text(encoding="utf-8"))):
            raw = match.group(1) if match.group(1) is not None else match.group(2)
            try:
                source = json.loads('"' + raw + '"')
            except json.JSONDecodeError:
                continue
            if re.search(r"\bfn\s+Main\s*\(", source):
                rows.append((f"{name}:{index}", source, J_PROFILE, None, None))
    for directory in ("pass", "fail", "objetivo"):
        for path in sorted((ROOT / "corpus" / directory).glob("*.n")):
            source = path.read_text(encoding="utf-8")
            profile = J_PROFILE if "F0-0003" in source.splitlines()[0] else A_PROFILE
            env = path.with_suffix(".env.json")
            rows.append((str(path.relative_to(ROOT)), source, profile, path,
                         env if env.is_file() else None))
    rd = json.loads((ROOT / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))
    rows.extend((f"RD:{row['id']}", row["source"], A_PROFILE, None, None)
                for row in rd["rows"])
    return rows


def invoke(binary: Path, command: str, path: Path, profile: str,
           env: Path | None) -> tuple[int, bytes, bytes, dict]:
    args = [str(binary), command, str(path), "--format", "json", "--profile", profile]
    if command == "run" and env:
        args.extend(["--mock-env", str(env)])
    done = subprocess.run(args, cwd=ROOT, capture_output=True, timeout=10, check=False)
    payload = json.loads(done.stdout)
    return done.returncode, done.stdout, done.stderr, payload


def main() -> int:
    suffix = ".exe" if os.name == "nt" else ""
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binaries = {mode: target / mode / f"nexo{suffix}"
                for mode in ("debug", "release")}
    for mode, binary in binaries.items():
        if not binary.is_file():
            raise SystemExit(f"missing {mode} binary: {binary}")
    rows = examples()
    failures = []
    with tempfile.TemporaryDirectory(prefix="nexo-j-diff-") as temporary:
        directory = Path(temporary)
        for index, (name, source, profile, path, env) in enumerate(rows):
            if path is None:
                path = directory / f"{index:04}.n"
                path.write_text(source, encoding="utf-8", newline="")
            results = {}
            for mode, binary in binaries.items():
                for command in ("check", "run"):
                    try:
                        results[(mode, command)] = invoke(binary, command, path, profile, env)
                    except (subprocess.TimeoutExpired, json.JSONDecodeError) as error:
                        failures.append((name, mode, command, type(error).__name__))
                        break
                if (mode, "check") not in results or (mode, "run") not in results:
                    continue
                checked = results[(mode, "check")][3]
                run = results[(mode, "run")][3]
                check_rejected = checked.get("outcome", {}).get("kind") == "rejected"
                run_rejected = run.get("outcome", {}).get("kind") == "rejected"
                if check_rejected != run_rejected or (check_rejected and checked["diagnostics"] != run["diagnostics"]):
                    failures.append((name, mode, "check/run", "acceptance or diagnostics"))
                if profile == J_PROFILE and any(
                    value.get("outcome", {}).get("reason") == "internal_error"
                    for value in (checked, run)
                ):
                    failures.append((name, mode, "internal_error", "driver"))
            for command in ("check", "run"):
                if ("debug", command) in results and ("release", command) in results:
                    if results[("debug", command)][:3] != results[("release", command)][:3]:
                        failures.append((name, command, "debug/release", "CLI bytes"))
    print(json.dumps({"sources": len(rows), "commands": len(rows) * 4,
                      "differences": len(failures), "first": failures[:12]}, ensure_ascii=False))
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
