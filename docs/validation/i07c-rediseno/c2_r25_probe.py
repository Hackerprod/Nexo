#!/usr/bin/env python3
"""Run source-level C2 design discriminators against two frozen binaries.

The expectations in the companion design are derived from the ADR, not from
either executable. This script records observations only.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

CASES = {
    "assignment_concrete": "fn Main(){let ar=[1];var r=[];let a=[r,[1]];r=[true];}",
    "literal_first": "fn Main(){var q=[];let a=[q,[1]];q=[true];}",
    "scalar_shape": "fn Main(){var q=[];let a=[q,1];}",
    "two_ctors_shape": "fn Main(){var q=[];var r=[];let a=[q,r,1];}",
    "block_if_join": "fn Main(){let a=(if true then {value: [1];} else {value: [true];});}",
    "block_if_first": "fn Main(){var q=[];let a=[(if true then {value: q;} else {value: [1];})];q=[true];}",
    "forward_type": "fn Main(){var r=[true];r=[1];}",
    "late_type": "fn Main(){var r=[];r=[(r==r)==true];r=[1];}",
}
FROZEN_IDS = {
    "r25_assignment_233": "b-projections-2026112401:233",
    "r25_origin_1995": "b-projections-2026112401:1995",
    "r25_prefix_761": "b-projections-t5-2026112402:761",
    "r25_literal_1390": "b-projections-2026112401:1390",
}
FROZEN = Path(__file__).with_name("c2-d8-iteration-pb.json")


def run(binary: Path, source: str):
    with tempfile.TemporaryDirectory(prefix="c2-r25-") as folder:
        path = Path(folder) / "case.n"
        path.write_text(source, encoding="utf-8")
        proc = subprocess.run(
            [str(binary.resolve()), "check", str(path), "--profile", "f0-0002-b-v2", "--format", "json"],
            capture_output=True, timeout=30, check=False,
        )
    if proc.returncode not in (0, 1):
        raise RuntimeError((proc.returncode, proc.stderr.decode("utf-8", "replace")))
    obj = json.loads(proc.stdout)
    return {
        "exit": proc.returncode,
        "outcome": obj.get("outcome", {}).get("kind"),
        "diagnostics": [
            {
                "code": d["code"],
                "subcause": d.get("subcause"),
                "primary": [d["primary_span"]["start"], d["primary_span"]["end"]],
                "token": source[d["primary_span"]["start"]:d["primary_span"]["end"]],
                "expected": d.get("data", {}).get("expected_type"),
                "found": d.get("data", {}).get("found_type"),
                "relations": [
                    [r["role"], r["span"]["start"], r["span"]["end"]]
                    for r in d.get("related_spans", [])
                ],
            }
            for d in obj.get("diagnostics", [])
        ],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--main", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    frozen = json.loads(FROZEN.read_text(encoding="utf-8"))
    cases = dict(CASES)
    cases.update({name: frozen["rows"][row_id]["source"] for name, row_id in FROZEN_IDS.items()})
    rows = {}
    for name, source in cases.items():
        rows[name] = {
            "source": source,
            "sha256": hashlib.sha256(source.encode()).hexdigest(),
            "main": run(args.main, source),
            "candidate": run(args.candidate, source),
        }
    args.output.write_text(json.dumps({"schema": "c2-r25-probes-v1", "rows": rows}, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(rows)} probes written to {args.output}")


if __name__ == "__main__":
    main()
