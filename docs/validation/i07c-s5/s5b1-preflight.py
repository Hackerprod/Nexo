"""Check that the archived baseline omits the selector's independent diagnostic.

Usage: python -B .../s5b1-preflight.py <base.exe> [<candidate.exe>]
"""

import json
from pathlib import Path
import subprocess
import sys
import tempfile


SOURCES = (
    ("fn Main(){let ar=[1];let z=ar[true][[1,true]];}", (30, 39)),
    ("fn Main(){let ar=[1];let z=(1+true)[[1,true]];}", (30, 39)),
    ("fn Main(){let ar=[1];let z=ar[true][2+true];}", (30, 38)),
    ("fn Main(){let ar=[1];let z=ar[true][0][[1,true]];}", (30, 42)),
)


def main():
    binaries = [Path(arg) for arg in sys.argv[1:]]
    assert 1 <= len(binaries) <= 2
    with tempfile.TemporaryDirectory(prefix="s5b1-preflight-") as directory:
        source_file = Path(directory) / "case.n"
        for source, (first, second) in SOURCES:
            source_file.write_text(source, encoding="ascii")
            for verb in ("check", "run"):
                for index, binary in enumerate(binaries):
                    result = subprocess.run((str(binary.resolve()), verb, str(source_file),
                                             "--profile", "f0-0002-b-v2", "--format", "json"),
                                            capture_output=True, text=True, timeout=30)
                    diagnostics = json.loads(result.stdout)["diagnostics"]
                    starts = [item["primary_span"]["start"] for item in diagnostics]
                    wanted = [first] if index == 0 else [first, second]
                    assert result.returncode == 1 and starts == wanted, (binary, source, starts, result.stderr)
                    print(f"{binary.name} {verb} {first},{second}: {starts}")


if __name__ == "__main__":
    main()
