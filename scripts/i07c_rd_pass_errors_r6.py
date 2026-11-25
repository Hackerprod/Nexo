#!/usr/bin/env python3
"""Reproduce exact CLI outcomes missed by the diagnostic-only sweep."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

import i07c_rd_sweep as sweep
from i07c_rd_drive_r6 import BASE, CANDIDATE, FLAGS


INDICES = (52, 12050, 14018, 15631)


def run(exe, source, command, flags):
    with tempfile.TemporaryDirectory(prefix="i07c-r6-pass-error-") as directory:
        file = Path(directory) / "p.n"
        file.write_text(source, encoding="utf-8")
        env = os.environ.copy()
        env.pop("NEXO_XV", None)
        if flags:
            env["NEXO_XV"] = flags
        process = subprocess.run([exe, command, "--format", "json", "--profile",
                                  "f0-0003-b-v1", str(file)], capture_output=True,
                                 text=True, env=env, timeout=60)
        return {"exit": process.returncode, "stderr": process.stderr,
                "outcome": json.loads(process.stdout)["outcome"]}


def main():
    sources = sweep.data("c2@f0-0003-b-v1")
    rows = []
    for index in INDICES:
        source = sweep.source(sources[index])
        readings = {name: {command: run(exe, source, command, flags)
                           for command in ("check", "run")}
                    for name, exe, flags in (("main", BASE, None),
                                             ("r6", CANDIDATE, FLAGS),
                                             ("r6_pass_counter", CANDIDATE, FLAGS + ",passes"))}
        rows.append({"index": index, "bytes": len(source.encode()),
                     "sha256": hashlib.sha256(source.encode()).hexdigest(),
                     "source": source, "readings": readings})
        print(index, len(source.encode()), {name: [readings[name][c]["outcome"]["kind"]
                                             for c in ("check", "run")] for name in readings})
    result = {"base_sha256": hashlib.sha256(Path(BASE).read_bytes()).hexdigest(),
              "candidate_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
              "flags": FLAGS, "rows": rows}
    path = Path("docs/validation/i07c-rediseno/r6-pass-errors-reproduced.json")
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
