"""Compare S3 against the frozen pre-S3 CLI on the protected source set."""

import argparse
import glob
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def sources(root):
    result = {}
    for pattern in ("corpus/pass/**/*.n", "corpus/fail/**/*.n", "corpus/objetivo/**/*.n", "tests/fixtures/**/*.n"):
        for name in sorted(glob.glob(str(root / pattern), recursive=True)):
            path = Path(name)
            result[str(path.relative_to(root)).replace("\\", "/")] = path.read_bytes()
    rows = json.loads((root / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))["rows"]
    for row in rows:
        result["RD:" + row["id"]] = row["source"].encode("utf-8")
    return result


def run(binary, mode, path):
    for attempt in range(2):
        try:
            result = subprocess.run(
                [str(binary), mode, "--profile", "f0-0003-b-v1", "--format", "json", str(path)],
                capture_output=True, timeout=120,
            )
            break
        except subprocess.TimeoutExpired:
            if attempt == 1:
                return ("timeout", b"", b"")
    path_bytes = str(path).encode()
    return (result.returncode, result.stdout.replace(path_bytes, b"SOURCE"),
            result.stderr.replace(path_bytes, b"SOURCE"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("base_debug", type=Path)
    parser.add_argument("base_release", type=Path)
    parser.add_argument("candidate_debug", type=Path)
    parser.add_argument("candidate_release", type=Path)
    args = parser.parse_args()
    binaries = {
        "base-debug": args.base_debug, "base-release": args.base_release,
        "candidate-debug": args.candidate_debug, "candidate-release": args.candidate_release,
    }
    root = Path(__file__).resolve().parents[2]
    cases = sources(root)
    assert len(cases) == 369, len(cases)
    digest = hashlib.sha256(b"".join(cases[name] for name in sorted(cases))).hexdigest()
    differences = []
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "source.n"
        for index, (name, source) in enumerate(cases.items(), 1):
            path.write_bytes(source)
            results = {(label, mode): run(binary, mode, path)
                       for label, binary in binaries.items() for mode in ("check", "run")}
            for profile in ("debug", "release"):
                for mode in ("check", "run"):
                    if results[("base-" + profile, mode)] != results[("candidate-" + profile, mode)]:
                        differences.append((name, profile, mode, "base/candidate"))
            for version in ("base", "candidate"):
                for mode in ("check", "run"):
                    if results[(version + "-debug", mode)] != results[(version + "-release", mode)]:
                        differences.append((name, version, mode, "debug/release"))
                for profile in ("debug", "release"):
                    try:
                        check = json.loads(results[(version + "-" + profile, "check")][1])["diagnostics"]
                        run_result = json.loads(results[(version + "-" + profile, "run")][1])["diagnostics"]
                    except (ValueError, KeyError):
                        differences.append((name, version, profile, "invalid JSON"))
                    else:
                        if check != run_result:
                            differences.append((name, version, profile, "check/run diagnostics"))
            for (label, mode), (code, out, err) in results.items():
                if code not in (0, 1) or err or any(term in out for term in (b"internal_error", b"InvalidIr", b"driver_error")):
                    differences.append((name, label, mode, "failed CLI", code, out[:200], err[:200]))
            if index % 25 == 0:
                print(f"{index}/{len(cases)} differences={len(differences)}", flush=True)
            if differences:
                break
    print(f"total sources={len(cases)} source_sha256={digest} differences={len(differences)}")
    for difference in differences[:10]:
        print(difference)
    if differences:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
