#!/usr/bin/env python3
"""Disposable focused probe for the I07c equality transaction experiment."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


SOURCES = {
    "t3584": "fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));r=q;q=[[ar]];}",
    "planned_min": "fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}",
    "planned_full": "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];var q=[];var s=[];var i0:Array<Array<Array<I64>>>=[[[0]]];i0[0]=([s[x],(if 0 then true else q),(if x then q else s)] == x);let a1=s;while Len(q) {}r=br;q=br;s=[[true]];}",
    "r4": "fn Main(){let x=1;let p=true;let ar=[1];var r=[];var q=[];let a0=(if p then q else (if p then 1 else r));q=[[ar]];}",
    "recursive": "fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}",
    "positive": "fn Main(){let p=true;let ar=[1];var q=[];let a=(if p then q else ar);q=ar;}",
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("exe")
    parser.add_argument("--flags", default=None)
    args = parser.parse_args()
    exe = str(Path(args.exe).resolve())
    env = os.environ.copy()
    if args.flags is not None:
        env["NEXO_XV"] = args.flags
    with tempfile.TemporaryDirectory(prefix="i07c-rd-") as directory:
        for name, source in SOURCES.items():
            filename = Path(directory) / f"{name}.n"
            filename.write_bytes(source.encode("ascii"))
            result = subprocess.run(
                [exe, "check", "--format", "json", "--profile", "f0-0002-b-v2", str(filename)],
                capture_output=True,
                text=True,
                env=env,
                timeout=60,
            )
            try:
                payload = json.loads(result.stdout)
                diagnostics = [
                    [d["code"], d.get("subcause"),
                     d.get("primary_span", {}).get("start"), d.get("primary_span", {}).get("end")]
                    for d in payload.get("diagnostics", [])
                ]
            except (ValueError, TypeError):
                diagnostics = ["INVALID_JSON", result.stdout[-300:], result.stderr[-300:]]
            print(json.dumps({"case": name, "exit": result.returncode, "diagnostics": diagnostics}))


if __name__ == "__main__":
    main()
