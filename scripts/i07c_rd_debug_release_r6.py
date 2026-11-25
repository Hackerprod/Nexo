#!/usr/bin/env python3
"""Compare debug/release CLI acceptance on frozen directed sources."""
import hashlib
import json
from pathlib import Path

from i07c_rd_b_minima import CASES
from i07c_rd_cheap import MINIMA
from i07c_rd_drive_r6 import FLAGS
from i07c_rd_profile_acceptance_r6 import inspect


def main():
    binaries = {name: Path(f"C:/nexo-work/target-r6/{name}/nexo.exe")
                for name in ("debug", "release")}
    sources = {**MINIMA, **{name: source for name, source, _ in CASES}}
    rows = []
    for profile in ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1"):
        for name, source in sources.items():
            readings = {mode: {command: inspect(str(binary), command, profile, source, FLAGS)
                               for command in ("check", "run")}
                        for mode, binary in binaries.items()}
            signature = lambda r: (r["exit"], r["outcome"], r["accepted"])
            rows.append({"profile": profile, "name": name,
                         "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                         "readings": readings,
                         "debug_release_same": all(signature(readings["debug"][command]) ==
                                                   signature(readings["release"][command])
                                                   for command in ("check", "run")),
                         "check_run_same": all(readings[mode]["check"]["accepted"] ==
                                               readings[mode]["run"]["accepted"]
                                               for mode in binaries)})
    document = {"binary_sha256": {name: hashlib.sha256(binary.read_bytes()).hexdigest()
                                   for name, binary in binaries.items()},
                "flags": FLAGS, "rows": rows,
                "debug_release_failures": [r["profile"] + "/" + r["name"] for r in rows
                                           if not r["debug_release_same"]],
                "check_run_failures": [r["profile"] + "/" + r["name"] for r in rows
                                       if not r["check_run_same"]]}
    dest = Path("docs/validation/i07c-rediseno/r6-debug-release-acceptance.json")
    dest.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"rows": len(rows), "debug_release_failures": len(document["debug_release_failures"]),
                      "check_run_failures": len(document["check_run_failures"])}))


if __name__ == "__main__":
    main()
