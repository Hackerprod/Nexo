#!/usr/bin/env python3
"""Freeze the r10 match-conservation probes from the pinned main binary."""

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
MATCH = HERE / "r8-match-main.json"
OUT = HERE / "r9-witness-discriminator.json"
PROFILE = "f0-0003-b-v1"
PREFIX = "fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];let a0=match move o{Some(n)=>{value: "
SUFFIX = ";},None=>{value: true;}};}"
COMBINATIONS = [
    ("1", "q", "q"), ("1", "q", "1"), ("1", "q", "2"),
    ("1", "q", "ar"), ("1", "ar", "q"), ("1", "ar", "ar"),
    ("1", "ar", "1"), ("ar", "1", "ar"), ("ar", "1", "1"),
    ("ar", "1", "2"), ("q", "1", "ar"), ("1", "2", "q"),
]


def compact(report):
    return {
        "outcome": report["outcome"]["kind"],
        "diagnosticos": [
            {
                "code": d["code"], "subcause": d.get("subcause"),
                "primary": [d["primary_span"]["start"], d["primary_span"]["end"]],
                "expected_type": d.get("data", {}).get("expected_type"),
                "found_type": d.get("data", {}).get("found_type"),
                "relations": [
                    {"role": r["role"], "span": [r["span"]["start"], r["span"]["end"]]}
                    for r in d.get("related_spans", [])
                ],
            }
            for d in report["diagnostics"]
        ],
    }


def run(exe, source, path):
    path.write_bytes(source.encode("ascii"))
    completed = subprocess.run(
        [str(exe), "check", "--format", "json", "--profile", PROFILE, str(path)],
        capture_output=True, text=True, timeout=20, check=False,
    )
    if completed.returncode not in (0, 1):
        raise RuntimeError((completed.returncode, completed.stderr))
    return compact(json.loads(completed.stdout))


def row(id_, source, expected, group):
    return {
        "id": id_, "fixture_id": id_, "profile": PROFILE,
        "source": source,
        "source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(),
        "conservation_class": group,
        "expected_main": expected,
        "expected_normative": None,
    }


def main():
    exe = Path(sys.argv[1]).resolve()
    if not exe.is_file():
        raise SystemExit(f"missing main binary: {exe}")
    frozen = json.loads(OUT.read_text(encoding="utf-8"))
    existing = frozen["rows"][:9]
    r10_followups = [row for row in frozen["rows"] if row["id"] in {
        "match-index-20", "match-index-46", "match-circular",
        "match-later-unassigned", "match-later-assigned",
    }]
    if len(r10_followups) != 5:
        raise RuntimeError("missing five r10 follow-up probes")
    matrix = json.loads(MATCH.read_text(encoding="utf-8"))["rows"]
    extra = []
    for index in (4, 44):
        m = matrix[index]
        extra.append(row(f"match-index-{index}", m["source"], m["expected_main"], "reported_outer_rejection"))
    with tempfile.TemporaryDirectory(prefix="nexo-r10-match-") as directory:
        path = Path(directory) / "probe.n"
        for number, (first, second, outer) in enumerate(COMBINATIONS, 1):
            source = PREFIX + f"(if p then (if p then {first} else {second}) else {outer})" + SUFFIX
            extra.append(row(f"match-conservation-{number:02}", source, run(exe, source, path), "borrowed_partial_probe"))
        source = PREFIX + "(if p then 1 else q)" + SUFFIX
        extra.append(row("match-conservation-flat", source, run(exe, source, path), "own_partial_without_borrow"))
    frozen["rows"] = existing + extra + r10_followups
    frozen["r10_main_binary_sha256"] = hashlib.sha256(exe.read_bytes()).hexdigest()
    OUT.write_text(json.dumps(frozen, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"frozen {len(extra)} r9 rows and retained five r10 follow-ups; total {len(frozen['rows'])}")


if __name__ == "__main__":
    main()
