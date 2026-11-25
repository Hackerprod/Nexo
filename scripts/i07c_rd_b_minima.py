#!/usr/bin/env python3
"""B's independent projection minima, with exact core expectations."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

M1 = "fn Main(){var q=[];var r=[];let b=q[0];(if true then [q] else (if true then [1] else q));r=q;q=[1];}"
M2 = "fn Main(){var q=[];var r=[];let z=r[0];let a=(if true then r[0] else (if true then q else r));r=q;r=[true];}"
CASES = [
    ("M1", M1, [("NX-E1303", 76, 79)]),
    ("M2", M2, [("NX-E0103", 16, 18)]),
]


def run(executable, source, profile, flags):
    with tempfile.TemporaryDirectory(prefix="rd-b-min-") as directory:
        path = Path(directory) / "p.n"
        path.write_bytes(source.encode("utf-8"))
        env = os.environ.copy()
        env.pop("NEXO_XV", None)
        if flags:
            env["NEXO_XV"] = flags
        process = subprocess.run(
            [executable, "check", "--format", "json", "--profile", profile, str(path)],
            capture_output=True, text=True, env=env, timeout=120,
        )
        result = json.loads(process.stdout)
        return [(item["code"], item["primary_span"]["start"], item["primary_span"]["end"])
                for item in result["diagnostics"]]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("main")
    parser.add_argument("candidate")
    parser.add_argument("--flags", required=True)
    args = parser.parse_args()
    failed = 0
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1"):
        for name, source, expected in CASES:
            base = run(args.main, source, profile, None)
            candidate = run(args.candidate, source, profile, args.flags)
            ok = base == expected and candidate == expected
            failed += not ok
            print(json.dumps({"profile": profile, "case": name, "expected": expected,
                              "main": base, "candidate": candidate, "ok": ok}))
    raise SystemExit(bool(failed))


if __name__ == "__main__":
    main()
