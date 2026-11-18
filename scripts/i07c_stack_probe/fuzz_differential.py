"""Replay the frozen D seed generator against pre-S3 and S3 binaries."""

import argparse
import contextlib
import hashlib
import io
import json
from pathlib import Path
import random
import runpy
import sys
import tempfile

from differential import run


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("generator", type=Path)
    parser.add_argument("base_debug", type=Path)
    parser.add_argument("base_release", type=Path)
    parser.add_argument("candidate_debug", type=Path)
    parser.add_argument("candidate_release", type=Path)
    parser.add_argument("--only-seed", type=int)
    parser.add_argument("--only-index", type=int)
    args = parser.parse_args()
    original_argv = sys.argv
    try:
        sys.argv = [str(args.generator), "1", "0"]
        with contextlib.redirect_stdout(io.StringIO()):
            generator = runpy.run_path(str(args.generator))
    finally:
        sys.argv = original_argv
    make_program = generator["prog"]
    binaries = {
        "base-debug": args.base_debug, "base-release": args.base_release,
        "candidate-debug": args.candidate_debug, "candidate-release": args.candidate_release,
    }
    digest = hashlib.sha256()
    differences = []
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "fuzz.n"
        for seed in range(1, 6):
            make_program.__globals__["rnd"] = random.Random(seed)
            for index in range(960):
                source = make_program().encode("ascii")
                digest.update(source)
                if args.only_seed is not None and (seed, index) != (args.only_seed, args.only_index):
                    continue
                path.write_bytes(source)
                results = {(label, mode): run(binary, mode, path)
                           for label, binary in binaries.items() for mode in ("check", "run")}
                for profile in ("debug", "release"):
                    for mode in ("check", "run"):
                        if results[("base-" + profile, mode)] != results[("candidate-" + profile, mode)]:
                            differences.append((seed, index, profile, mode, "base/candidate", source))
                for version in ("base", "candidate"):
                    for mode in ("check", "run"):
                        if results[(version + "-debug", mode)] != results[(version + "-release", mode)]:
                            differences.append((seed, index, version, mode, "debug/release", source))
                    for profile in ("debug", "release"):
                        try:
                            check = json.loads(results[(version + "-" + profile, "check")][1])["diagnostics"]
                            run_result = json.loads(results[(version + "-" + profile, "run")][1])["diagnostics"]
                        except (ValueError, KeyError):
                            differences.append((seed, index, version, profile, "invalid JSON", source))
                        else:
                            if check != run_result:
                                differences.append((seed, index, version, profile, "check/run diagnostics", source))
                for (label, mode), (code, out, err) in results.items():
                    if code not in (0, 1) or err or any(term in out for term in (b"internal_error", b"InvalidIr", b"driver_error")):
                        differences.append((seed, index, label, mode, "failed CLI", source, code, out[:200], err[:200]))
                if (index + 1) % 240 == 0:
                    print(f"seed={seed} sources={index + 1}/960 differences={len(differences)}", flush=True)
                if differences:
                    for key, value in results.items():
                        print(f"result {key}: {value}", flush=True)
                    break
            if differences:
                break
    print(f"source_sha256={digest.hexdigest()} differences={len(differences)}")
    for difference in differences[:5]:
        print(difference)
    if differences:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
