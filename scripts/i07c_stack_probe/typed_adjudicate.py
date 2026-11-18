"""Compare reduced typed-campaign witnesses with the frozen pre-S3 CLI."""

import argparse
import hashlib
import os
from pathlib import Path
import re
import subprocess
import tempfile

from differential import run


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=Path)
    parser.add_argument("seed", type=int)
    parser.add_argument("harness", type=Path)
    parser.add_argument("base_debug", type=Path)
    parser.add_argument("base_release", type=Path)
    parser.add_argument("candidate_debug", type=Path)
    parser.add_argument("candidate_release", type=Path)
    parser.add_argument("--minimum-index", type=int, default=0)
    args = parser.parse_args()
    indices = sorted({int(value) for value in re.findall(
        r"idx=([0-9]+)", args.log.read_text(encoding="utf-8"))
        if int(value) >= args.minimum_index})
    binaries = {
        "base-debug": args.base_debug, "base-release": args.base_release,
        "candidate-debug": args.candidate_debug,
        "candidate-release": args.candidate_release,
    }
    digest = hashlib.sha256()
    differences = []
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "typed.n"
        for index in indices:
            env = os.environ.copy()
            env["L2"] = "1"
            shown = subprocess.run(
                [str(args.harness), "reduce-show", str(index), str(args.seed), "8", "3"],
                env=env, capture_output=True, text=True, check=True, timeout=60,
            ).stdout
            sources = re.findall(
                r"^(deferred|concrete|permutation \[[^]]+\]): (.*)$", shown, re.M,
            )
            assert sources, (args.seed, index)
            for label, source in sources:
                data = source.encode("ascii")
                digest.update(data)
                path.write_bytes(data)
                results = {(name, mode): run(binary, mode, path)
                           for name, binary in binaries.items()
                           for mode in ("check", "run")}
                for profile in ("debug", "release"):
                    for mode in ("check", "run"):
                        if results[("base-" + profile, mode)] != results[("candidate-" + profile, mode)]:
                            differences.append((index, label, profile, mode, source))
            print(f"seed={args.seed} index={index} variants={len(sources)} differences={len(differences)}", flush=True)
            if differences:
                break
    print(f"indices={len(indices)} reduced_source_sha256={digest.hexdigest()} differences={len(differences)}")
    for difference in differences:
        print(difference)
    if differences:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
