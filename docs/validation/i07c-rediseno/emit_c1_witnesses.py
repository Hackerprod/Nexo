#!/usr/bin/env python3
"""Freeze the shortest c1 witnesses for independent ADR review by B."""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

from audit_families import HERE, oracle, project_main, project_oracle, source

IDS = {
    "main_extra_diagnostic": [44, 10707, 5703, 12348],
    "main_missing_diagnostic": [13202, 3434, 4081],
    "different_code_or_primary": [6193, 12890, 1425],
    "different_type_or_relation": [8476, 6934, 22309],
    "provenance_relation": [8381, 10345, 16811],
}


def main():
    executable, case_path, output_path = map(Path, sys.argv[1:4])
    rows = json.loads((HERE / "c1.json").read_text(encoding="utf-8"))
    if isinstance(rows, dict):
        rows = rows["progs"]
    result = {"schema": "i07c-r12-c1-witnesses-v1",
              "main_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
              "main_binary_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
              "c1_sha256": hashlib.sha256((HERE / "c1.json").read_bytes()).hexdigest(),
              "note": "Observed oracle/main projections; normative expectations and derivations are hand transcribed in r12-family-table.md.",
              "families": {}}
    for name, ids in IDS.items():
        witnesses = []
        for index in ids:
            src = source(rows[index])
            case_path.write_bytes(src.encode("utf-8"))
            run = subprocess.run([str(executable), "check", str(case_path), "--profile",
                                  "f0-0002-b-v2", "--format", "json"],
                                 capture_output=True, timeout=20, check=False)
            assert run.returncode in (0, 1), (index, run.returncode, run.stderr)
            derived = oracle.derive(src)
            assert not any(derived.get(k) for k in ("error", "unmodeled", "no_fijado", "d_optional"))
            witnesses.append({"id": f"c1:{index}", "bytes": len(src.encode("utf-8")),
                              "source": src, "oracle_observed": project_oracle(derived),
                              "main_observed": project_main(json.loads(run.stdout))})
        result["families"][name] = witnesses
    output_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
