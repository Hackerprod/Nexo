#!/usr/bin/env python3
"""Run the R1 T5 correction and R3 transaction probes sequentially."""
import subprocess
import sys


def main():
    exe = sys.argv[1]
    for script, output in (
        ("scripts/i07c_rd_r1_probe.py", "docs/validation/i07c-rediseno/r1-t5-correccion-sondas.json"),
        ("scripts/i07c_rd_r3_probe.py", "docs/validation/i07c-rediseno/r3-sondas.json"),
    ):
        subprocess.run([sys.executable, "-B", script, exe, output], check=True)


if __name__ == "__main__":
    main()
