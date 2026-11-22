"""Additional TYPES probes at 129/204/256 KiB, five runs per case and size."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]


def main():
    build = subprocess.run(("cargo", "test", "--lib", "--no-run", "--profile", "test",
                            "--message-format", "json", "--locked", "--offline"),
                           cwd=ROOT, capture_output=True, text=True, timeout=300)
    assert build.returncode == 0, build.stderr
    artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
    executable = [Path(artifact["executable"]) for artifact in artifacts
                  if artifact.get("reason") == "compiler-artifact"
                  and artifact["target"]["name"] == "nexo"
                  and "lib" in artifact["target"]["kind"]
                  and artifact.get("executable")]
    assert len(executable) == 1
    print("exe_sha256=" + hashlib.sha256(executable[0].read_bytes()).hexdigest())
    probe = ROOT / "scripts/i07c_stack_probe/probe.py"
    for case in ("option_match126", "result_match126"):
        for kib in (129, 204, 256):
            for trial in range(1, 6):
                result = subprocess.run((sys.executable, "-B", str(probe), str(executable[0]),
                                         "--mode", "TYPES", "--kib", str(kib), "--case", case),
                                        cwd=ROOT, capture_output=True, text=True, timeout=180)
                assert result.returncode == 0, (case, kib, trial, result.stdout, result.stderr)
                print(f"{case} {kib} KiB {trial}/5 exit=0", flush=True)


if __name__ == "__main__":
    main()
