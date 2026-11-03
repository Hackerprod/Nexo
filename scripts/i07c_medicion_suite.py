#!/usr/bin/env python3
"""Ejecuta ambas suites y deriva sus cifras de las salidas de Cargo."""

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/i07c-suite-results.json"
RUNNING = re.compile(r"Running (?:unittests|tests)[ /\\]([^ (]+)\.rs")
RESULT = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored")


def suite(profile: str, log: Path) -> dict:
    command = ["cargo", "test", "--locked", "--offline"]
    if profile == "release":
        command.append("--release")
    done = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          check=False)
    data = done.stdout
    log.write_bytes(data)
    text = data.decode("utf-8", "replace")
    binaries = {}
    current = None
    for line in text.splitlines():
        running = RUNNING.search(line)
        if running:
            current = running.group(1).replace("\\", "/")
            continue
        result = RESULT.search(line)
        if result and current is not None:
            assert result.group(1) == "ok" and result.group(3) == "0", (profile, current, line)
            binaries[current] = {"passed": int(result.group(2)), "ignored": int(result.group(4))}
            current = None
    assert done.returncode == 0, f"{profile}: {log}"
    assert "src/lib" in binaries and "i07c_rd_conformidad" in binaries, binaries
    assert binaries["i07c_rd_conformidad"]["passed"] == 1
    rows = json.loads((ROOT / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))["rows"]
    assert len(rows) == 152
    return {
        "command": command,
        "exit_code": done.returncode,
        "log_sha256": hashlib.sha256(data).hexdigest(),
        "binaries": {"src/lib.rs" if key == "src/lib" else "src/main.rs" if key == "src/main" else key: value
                     for key, value in binaries.items()},
        "rd": f"{len(rows)}/{len(rows)} check/run pairs normativos",
    }


def recorded_quiet_suite(profile: str, log: Path, all_targets: bool = False) -> dict:
    """Derive the measured binary table from an already completed quiet run."""
    data = log.read_bytes()
    text = data.decode("utf-16" if data.startswith(b"\xff\xfe") else "utf-8")
    observations = []
    pending = None
    for line in text.splitlines():
        running = re.fullmatch(r"running (\d+) tests?", line)
        if running:
            assert pending is None, (profile, line)
            pending = int(running.group(1))
            continue
        result = RESULT.search(line)
        if result:
            assert pending is not None and result.group(1) == "ok" and result.group(3) == "0"
            passed, ignored = int(result.group(2)), int(result.group(4))
            assert pending == passed + ignored, (profile, pending, line)
            observations.append({"passed": passed, "ignored": ignored})
            pending = None
    assert pending is None
    names = ["src/lib.rs", "src/main.rs"]
    names.extend(path.stem for path in sorted((ROOT / "tests").glob("*.rs")))
    assert len(observations) == len(names) + (0 if all_targets else 1), (
        profile, len(observations), names
    )
    if not all_targets:
        assert observations[-1] == {"passed": 0, "ignored": 0}, "doctests finales"
        observations = observations[:-1]
    binaries = dict(zip(names, observations, strict=True))
    assert binaries["i07c_rd_conformidad"] == {"passed": 1, "ignored": 0}
    assert "RD normative: 152/152 check/run pairs" in text
    rows = json.loads((ROOT / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))["rows"]
    assert len(rows) == 152
    command = ["cargo", "test", *(["--release"] if profile == "release" else []),
               *(["--all-targets"] if all_targets else []), "--locked", "--offline", "--quiet"]
    return {"command": command, "exit_code": 0,
            "log_sha256": hashlib.sha256(data).hexdigest(),
            "binaries": binaries,
            "rd": "152/152 check/run pairs normativos"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log-dir", type=Path)
    parser.add_argument("--from-quiet-log-dir", type=Path,
                        help="resume dos ejecuciones --quiet ya terminadas, sin repetirlas")
    parser.add_argument("--from-all-targets-log-dir", type=Path,
                        help="resume dos ejecuciones --all-targets --quiet ya terminadas")
    args = parser.parse_args()
    assert sum(value is not None for value in (
        args.log_dir, args.from_quiet_log_dir, args.from_all_targets_log_dir
    )) == 1
    folder = (args.log_dir or args.from_quiet_log_dir or args.from_all_targets_log_dir).resolve()
    assert ROOT == folder or ROOT in folder.parents
    if args.log_dir is not None:
        folder.mkdir(parents=True, exist_ok=True)
        profiles = {name: suite(name, folder / f"suite-{name}.log") for name in ("debug", "release")}
    else:
        all_targets = args.from_all_targets_log_dir is not None
        pattern = ("i07c-all-targets-{}.log" if all_targets else
                   "i07c-cargo-test-{}-final.log")
        profiles = {name: recorded_quiet_suite(name, folder / pattern.format(name), all_targets)
                    for name in ("debug", "release")}
    OUTPUT.write_text(json.dumps({"profiles": profiles}, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print({name: row["binaries"]["src/lib.rs"] for name, row in profiles.items()})


if __name__ == "__main__":
    main()
