#!/usr/bin/env python3
"""Resume todas las pruebas ignoradas en release con cuatro y un hilo."""

import argparse
import hashlib
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/validation/i07c-ignored-results.json"


def campaign_sizes() -> dict[str, int]:
    source = (ROOT / "tests/i07c_deferred_metamorphic.rs").read_text(encoding="utf-8")
    rhs = re.search(
        r"fn poisoned_assignment_and_independent_let_have_identical_rhs_causes\(\).*?for number in 1\.\.=(\d[\d_]*)",
        source, re.S,
    )
    twin = re.search(
        r"fn later_array_assignment_and_concrete_initializer_have_identical_causes\(\).*?for \(seed, count\) in \[(.*?)\]",
        source, re.S,
    )
    assert rhs and twin
    twin_counts = re.findall(r"\(\d+,\s*(\d[\d_]*)\)", twin.group(1))
    assert twin_counts
    return {"rhs": int(rhs.group(1).replace("_", "")),
            "twins": sum(int(value.replace("_", "")) for value in twin_counts)}


def parse(path: Path, expected: int) -> tuple[list[str], dict]:
    data = path.read_bytes()
    log = data.decode("utf-16" if data.startswith(b"\xff\xfe") else "utf-8", "replace")
    names = re.findall(r"(?m)^test ([^\r\n]+?) \.\.\. ", log)
    assert len(names) == len(set(names)) == expected, (path, len(names))
    summaries = re.findall(r"(?m)^test result: ok\. (\d+) passed; 0 failed; \d+ ignored;", log)
    assert sum(map(int, summaries)) == expected, (path, summaries)
    assert not re.search(r"(?m)^test [^\r\n]+ \.\.\. FAILED$", log), path
    deterministic = []
    clock = []
    for line in log.splitlines():
        ratios = [float(value) for value in re.findall(
            r"\b(?:ratio|work)\s*(?:=|:)?\s*(\d+\.\d+)\b", line
        )]
        if "sparse family" in line and re.search(r"\bmetric 3\b", line):
            clock.extend(ratios)
        else:
            deterministic.extend(ratios)
    assert max(deterministic, default=0.0) <= 4.6, path
    return names, {"tests": expected,
                   "max_scale_ratio": max(deterministic, default=0.0),
                   "max_clock_ratio": max(clock, default=0.0)}


def combined(path: Path, extra: Path | None, rhs: Path | None,
             twin: Path | None, scale: Path | None, threads: int,
             library_expected: int, sizes: dict[str, int]) -> dict:
    components = [parse(path, library_expected)]
    if extra:
        components.append(parse(extra, 1))
    if rhs:
        components.append(parse(rhs, 1))
    if twin:
        components.append(parse(twin, 1))
    names = [name for group, _ in components for name in group]
    expected = library_expected + int(extra is not None) + int(rhs is not None) + int(twin is not None)
    assert len(names) == len(set(names)) == expected
    if scale:
        raw = scale.read_bytes()
        log = raw.decode("utf-16" if raw.startswith(b"\xff\xfe") else "utf-8", "replace")
        deterministic = []
        clock = []
        for line in log.splitlines():
            match = re.search(r"(?:sparse )?family \d+ metric (\d+):", line)
            if not match:
                continue
            value = re.search(r"(?:\bratio\s*|=\s*)(\d+\.\d+)\b", line)
            if value:
                (clock if "sparse family" in line and match.group(1) == "3" else deterministic).append(float(value.group(1)))
        assert deterministic and clock and max(deterministic) <= 4.6, scale
        components.append(([], {"tests": 0, "max_scale_ratio": max(deterministic),
                                "max_clock_ratio": max(clock)}))
    assert max(component["max_scale_ratio"] for _, component in components) > 0
    assert max(component["max_clock_ratio"] for _, component in components) > 0
    source = (ROOT / "src/check/types.rs").read_bytes().replace(b"\r\n", b"\n")
    input_digest = hashlib.sha256(source + "\n".join(sorted(names)).encode()).hexdigest()
    return {
        "threads": threads,
        "profile": "release",
        "tests": sorted(names),
        "informative_tests": sorted(name for name in names
                                    if "quota_property::reduced_quota_property" in name
                                    or "generated_multifunction_quota" in name),
        "passed": len(names),
        "failed": 0,
        "input_sha256": input_digest,
        "max_scale_ratio": max(component["max_scale_ratio"] for _, component in components),
        "max_clock_ratio": max(component["max_clock_ratio"] for _, component in components),
        "components": [component for _, component in components],
        "campaign_programs": {"rhs": sizes["rhs"] if rhs else 0,
                              "twins": sizes["twins"] if twin else 0},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parallel-log", type=Path, required=True)
    parser.add_argument("--parallel-extra-log", type=Path)
    parser.add_argument("--single-log", type=Path, required=True)
    parser.add_argument("--single-extra-log", type=Path)
    parser.add_argument("--parallel-rhs-log", type=Path)
    parser.add_argument("--single-rhs-log", type=Path)
    parser.add_argument("--parallel-twin-log", type=Path)
    parser.add_argument("--single-twin-log", type=Path)
    parser.add_argument("--parallel-scale-log", type=Path)
    parser.add_argument("--single-scale-log", type=Path)
    args = parser.parse_args()
    assert (args.parallel_extra_log is None) == (args.single_extra_log is None)
    assert (args.parallel_rhs_log is None) == (args.single_rhs_log is None)
    assert (args.parallel_twin_log is None) == (args.single_twin_log is None)
    assert (args.parallel_scale_log is None) == (args.single_scale_log is None)
    suites = json.loads((ROOT / "docs/validation/i07c-suite-results.json").read_text(encoding="utf-8"))["profiles"]
    library_expected = suites["release"]["binaries"]["src/lib.rs"]["ignored"]
    sizes = campaign_sizes()
    parallel = combined(args.parallel_log, args.parallel_extra_log,
                        args.parallel_rhs_log, args.parallel_twin_log,
                        args.parallel_scale_log, 4, library_expected, sizes)
    single = combined(args.single_log, args.single_extra_log,
                      args.single_rhs_log, args.single_twin_log,
                      args.single_scale_log, 1, library_expected, sizes)
    assert parallel["tests"] == single["tests"]
    OUTPUT.write_text(
        json.dumps({"runs": [parallel, single]}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"Pruebas ignoradas registradas: {len(parallel['tests'])}, en ambos modos")


if __name__ == "__main__":
    main()
