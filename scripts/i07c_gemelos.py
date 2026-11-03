#!/usr/bin/env python3
"""Ejecuta pares diferido/concreto con la misma variable r y guarda evidencia."""

import argparse
import json
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HARNESS = ROOT / "scripts/i07c_gemelos/Cargo.toml"
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / ".scratch/i07c-gemelos-target"))
OUTPUT = ROOT / "docs/validation"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--programs", type=int, default=300_000)
    parser.add_argument("--stride", type=int, default=1)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    args = parser.parse_args()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TARGET)
    build = ["cargo", "build", "--offline", "--manifest-path", str(HARNESS)]
    if args.profile == "release":
        build.append("--release")
    subprocess.run(build, cwd=ROOT, env=env, check=True)
    suffix = ".exe" if os.name == "nt" else ""
    binary = TARGET / args.profile / ("i07c-gemelos" + suffix)
    command = [str(binary), "pairs", str(args.programs), str(args.stride),
               str(args.offset), str(args.threads)]
    done = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, check=True)
    lines = done.stdout.decode("utf-8").splitlines()
    summary = next(line for line in lines if line.startswith("programs="))
    integrity = next(line for line in lines if line.startswith("dup_strict="))
    exemptions = []
    citations = {
        "array elements checked against a deferred element type": "§4.1.2.7, RD01/RD02",
        "deferred binding mismatch": "§4.1.2.7, RD07/RD08",
        "r elem-level mismatch": "§4.1.2.7, RD17/RD18",
        "later assignment conflicts with accepted prefix equality": "§4.1.2.7, RD07/RD08",
    }
    for line in lines:
        if line.startswith("EXEMPT\t"):
            _, reason, count, differences = line.split("\t")
            exemptions.append({"reason": reason, "programs": int(count),
                               "differences": int(differences), "adr": citations[reason]})
    fields = {key: int(value) for key, value in
              re.findall(r"([a-z_0-9]+)=([0-9]+)", summary + " " + integrity)}
    print(summary, flush=True)
    print(integrity, flush=True)
    if fields["twin_diff"]:
        for line in (line for line in lines if line.startswith("EX[twin_diff]")):
            print(line, flush=True)
    assert fields["programs"] == args.programs
    assert args.programs >= 300_000, "la puerta exige al menos 300.000 programas"
    assert sum(row["programs"] for row in exemptions) == fields["twin_exempt"]
    assert sum(row["differences"] for row in exemptions) == fields["twin_exempt_diff"]
    assert fields["twin_diff"] == 0, "diferencias causales entre gemelos comparables"
    assert fields["bug"] == 0 and fields["dup_strict"] == 0 and fields["dup_site"] == 0
    result = {"informative": True,
              "profile": args.profile, "seed": args.offset, "stride": args.stride,
              "threads": args.threads, "programs": args.programs,
              "comparable_twins": fields["twin_checked"],
              "deferred_resolution_exemptions": fields["twin_exempt"],
              "deferred_resolution_differences": fields["twin_exempt_diff"],
              "exemption_families": exemptions,
              "differences": fields["twin_diff"],
              "duplicates": fields["dup_strict"] + fields["dup_site"],
              "bugs": fields["bug"]}
    output = OUTPUT / ("i07c-gemelos.json" if args.profile == "release"
                       else "i07c-gemelos-debug.json")
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8", newline="\n")
    print(f"{args.programs} programas, {fields['twin_checked']} gemelos, "
          f"{fields['twin_diff']} diferencias")


if __name__ == "__main__":
    main()
