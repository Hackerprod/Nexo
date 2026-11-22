"""Compare S5b.1 binaries using a previously frozen selector corpus.

Usage: python -B .../s5b1-differential.py <base-debug.exe> <base-release.exe>
           <candidate-debug.exe> <candidate-release.exe> <output.json> [start count]
"""

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
PROFILES = ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1")


def sources():
    module_spec = importlib.util.spec_from_file_location("s5b1_corpus", HERE / "s5b1-corpus.py")
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    generated = module.generated()
    frozen = json.loads((HERE / "s5b1-corpus.json").read_text(encoding="utf-8"))
    assert frozen["seed"] == module.SEED
    assert frozen["count"] == len(generated) == 3000
    assert frozen["sha256"] == module.digest(generated)
    yield from ((f"generated:{i:04d}", source) for i, source in enumerate(generated))
    for directory in (ROOT / "corpus", ROOT / "tests/fixtures"):
        for file in sorted(directory.rglob("*.n")):
            yield str(file.relative_to(ROOT)).replace("\\", "/"), file.read_text(encoding="utf-8")
    spec = importlib.util.spec_from_file_location("i07c_rd", ROOT / "scripts/i07c_rd_conformidad.py")
    rd = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rd)
    _, rows = rd.table()
    assert len(rows) == 152
    yield from (("rd:" + row["id"], row["source"]) for row in rows)


def call(binary, verb, profile, file):
    process = subprocess.run((str(binary), verb, str(file), "--format", "json", "--profile", profile),
                             capture_output=True, timeout=30)
    stdout = process.stdout.decode("utf-8", errors="replace").replace(str(file), "case.n")
    stderr = process.stderr.decode("utf-8", errors="replace").replace(str(file), "case.n")
    return process.returncode, stdout, stderr


def main():
    if len(sys.argv) not in (6, 8):
        raise SystemExit(__doc__)
    base_debug, base_release, current_debug, current_release = map(Path, sys.argv[1:5])
    output = Path(sys.argv[5])
    start, count = (int(sys.argv[6]), int(sys.argv[7])) if len(sys.argv) == 8 else (0, None)
    binaries = (base_debug, base_release, current_debug, current_release)
    assert all(binary.is_file() for binary in binaries)
    identity = {binary.name + ":" + str(i): hashlib.sha256(binary.read_bytes()).hexdigest()
                for i, binary in enumerate(binaries)}
    differences = []
    totals = {"sources": 0, "invocations": 0, "changes": 0}
    with tempfile.TemporaryDirectory(prefix="s5b1-diff-") as directory:
        file = Path(directory) / "case.n"
        for offset, (identifier, source) in enumerate(sources()):
            if offset < start:
                continue
            if count is not None and offset >= start + count:
                break
            totals["sources"] += 1
            file.write_text(source, encoding="utf-8")
            for profile in PROFILES:
                for verb in ("check", "run"):
                    for mode, base, current in (("debug", base_debug, current_debug),
                                                ("release", base_release, current_release)):
                        try:
                            old, new = call(base, verb, profile, file), call(current, verb, profile, file)
                        except subprocess.TimeoutExpired:
                            print(f"TIMEOUT source_id={identifier} source={source!r} "
                                  f"profile={profile} verb={verb} mode={mode}", flush=True)
                            raise
                        totals["invocations"] += 2
                        if old != new:
                            totals["changes"] += 1
                            differences.append({"source_id": identifier, "source": source,
                                                "profile": profile, "verb": verb, "mode": mode,
                                                "base": old, "candidate": new})
            if totals["sources"] % 100 == 0:
                print(f"sources={totals['sources']} changes={totals['changes']}", flush=True)
    report = {"manifest": json.loads((HERE / "s5b1-corpus.json").read_text()),
              "range": [start, start + totals["sources"]],
              "binary_sha256": identity, "totals": totals, "differences": differences}
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"saved={output} totals={totals}")


if __name__ == "__main__":
    main()
