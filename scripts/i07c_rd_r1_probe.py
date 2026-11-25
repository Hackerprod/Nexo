#!/usr/bin/env python3
"""Reproduce the disposable R1 cheap matrix with a hashed binary."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


BASE = "pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr,skipv3"
VARIANTS = {
    "obligacion_activa": "r1ledger",
    "historial": "r1ledger,r1history",
    "historial_t5": "r1ledger,r1history,r1t5",
}


def main():
    exe = Path(sys.argv[1])
    destination = Path(sys.argv[2])
    digest = hashlib.sha256(exe.read_bytes()).hexdigest()
    result = {"binary_sha256": digest, "variants": {}}
    for name, switches in VARIANTS.items():
        flags = BASE + "," + switches
        run = subprocess.run(
            [sys.executable, "-B", "scripts/i07c_rd_cheap.py", str(exe), flags],
            capture_output=True, text=True, timeout=21 * 60, check=True,
        )
        row = json.loads(run.stdout)
        row["binary_sha256"] = digest
        result["variants"][name] = row
        destination.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(name, row["summary"], flush=True)
        if any("inconclusive" in item for group in (row["probes"], row["matrix"])
               for item in group.values()):
            raise SystemExit("INCONCLUSO: repetir variante aislada")


if __name__ == "__main__":
    main()
