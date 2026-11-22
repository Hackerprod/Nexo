"""Snapshot debug/release binaries of the archived HEAD before candidate build.

Usage: python -B .../s5b1-snapshot.py <target-dir> <destination-dir>
"""

import hashlib
import json
from pathlib import Path
import shutil
import sys


def main():
    target, destination = map(Path, sys.argv[1:])
    destination.mkdir(exist_ok=False)
    digests = {}
    for mode in ("debug", "release"):
        source = target / mode / "nexo.exe"
        dest = destination / f"base-{mode}.exe"
        shutil.copyfile(source, dest)
        digests[mode] = hashlib.sha256(dest.read_bytes()).hexdigest()
    (destination / "hashes.json").write_text(json.dumps(digests, indent=2) + "\n")
    print(digests)


if __name__ == "__main__":
    main()
