#!/usr/bin/env python3
"""C0 gate: byte-exact main/candidate check+run across frozen corpus and match rows."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))
from audit_families import source  # noqa: E402
from verify_corpora import FILE  # noqa: E402


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def invoke(binary, verb, case, profile):
    return subprocess.run(
        [str(binary), verb, str(case), "--profile", profile, "--format", "json"],
        cwd=ROOT,
        capture_output=True,
        timeout=30,
        check=False,
    )


def same(main, candidate):
    return (main.returncode, main.stdout, main.stderr) == (
        candidate.returncode,
        candidate.stdout,
        candidate.stderr,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-debug", type=Path, required=True)
    parser.add_argument("--main-release", type=Path, required=True)
    parser.add_argument("--candidate-debug", type=Path, required=True)
    parser.add_argument("--candidate-release", type=Path, required=True)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    subprocess.run([sys.executable, "-B", str(HERE / "verify_corpora.py")], cwd=ROOT, check=True)
    binaries = {
        "debug": (args.main_debug, args.candidate_debug),
        "release": (args.main_release, args.candidate_release),
    }
    report = {
        "schema": "i07c-c0-exact-main-v1",
        "complete": True,
        "main_sha256": {k: sha(v[0]) for k, v in binaries.items()},
        "candidate_sha256": {k: sha(v[1]) for k, v in binaries.items()},
        "corpus": {"rows": 0, "invocations": 0, "mismatches": []},
        "match103": {"rows": 0, "invocations": 0, "mismatches": []},
    }

    rows = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    count = 0
    for entry in rows:
        ident = entry["id"]
        data = json.loads((HERE / FILE[ident.split("@")[0]]).read_text(encoding="utf-8"))
        if isinstance(data, dict):
            data = data.get("progs", data.get("rows", data))
        for index, item in enumerate(data):
            src = source(item)
            args.case.write_text(src, encoding="utf-8", newline="")
            for mode, (base, candidate) in binaries.items():
                for verb in ("check", "run"):
                    before = invoke(base, verb, args.case, entry["profile"])
                    after = invoke(candidate, verb, args.case, entry["profile"])
                    report["corpus"]["invocations"] += 2
                    if not same(before, after):
                        report["corpus"]["mismatches"].append(
                            {"id": f"{ident}:{index}", "mode": mode, "verb": verb,
                             "main_exit": before.returncode, "candidate_exit": after.returncode,
                             "main_stdout_sha256": hashlib.sha256(before.stdout).hexdigest(),
                             "candidate_stdout_sha256": hashlib.sha256(after.stdout).hexdigest()}
                        )
            count += 1
            if count % 1000 == 0:
                print(f"corpus rows={count}/{sum(r['rows'] for r in rows)} mismatches={len(report['corpus']['mismatches'])}", flush=True)
    report["corpus"]["rows"] = count

    match = json.loads((HERE / "r8-match-main.json").read_text(encoding="utf-8"))["rows"]
    if len(match) != 103:
        raise ValueError(f"expected 103 match rows, got {len(match)}")
    for index, row in enumerate(match):
        args.case.write_text(row["source"], encoding="utf-8", newline="")
        for mode, (base, candidate) in binaries.items():
            for verb in ("check", "run"):
                before = invoke(base, verb, args.case, row["profile"])
                after = invoke(candidate, verb, args.case, row["profile"])
                report["match103"]["invocations"] += 2
                if not same(before, after):
                    report["match103"]["mismatches"].append(
                        {"index": index, "mode": mode, "verb": verb,
                         "main_exit": before.returncode, "candidate_exit": after.returncode,
                         "main_stdout_sha256": hashlib.sha256(before.stdout).hexdigest(),
                         "candidate_stdout_sha256": hashlib.sha256(after.stdout).hexdigest()}
                    )
    report["match103"]["rows"] = len(match)
    report["script_sha256"] = sha(__file__)
    report["corpus_manifest_sha256"] = sha(HERE / "r7-frozen-batteries.json")
    report["match_manifest_sha256"] = sha(HERE / "r8-match-main.json")
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"corpus_rows": count, "corpus_invocations": report["corpus"]["invocations"],
                      "corpus_mismatches": len(report["corpus"]["mismatches"]),
                      "match_rows": len(match), "match_invocations": report["match103"]["invocations"],
                      "match_mismatches": len(report["match103"]["mismatches"])}), flush=True)
    if report["corpus"]["mismatches"] or report["match103"]["mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
