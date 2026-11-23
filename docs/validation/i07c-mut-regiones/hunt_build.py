"""Builds the release `nexo.exe` of one population mutant in a disposable copy and restores the file.

Usage: hunt_build.py <tree> <population.json> <id> <out_exe> <target_dir>
SHA-256 initial / mutated / restored are checked and printed as JSON. The base exe is built the same
way with id `BASE` (no mutation).
"""
import importlib.util, json, os, shutil, subprocess, sys
from pathlib import Path

tree, pop, ident, out, target = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]), sys.argv[5]
spec = importlib.util.spec_from_file_location("mv", Path(__file__).resolve().parents[3] / "scripts/mutant-verdict.py")
mv = importlib.util.module_from_spec(spec); spec.loader.exec_module(mv)
env = dict(os.environ, CARGO_TARGET_DIR=target)
row = {"id": ident}
m = None
if ident != "BASE":
    m = next(x for x in json.loads(pop.read_text(encoding="utf-8"))["mutations"] if x["id"] == ident)
    path = tree / m["file"]
    original = path.read_bytes()
    row["sha_initial"] = mv.digest(original)
    mutated = mv.replace_nth(original.decode("utf-8"), m["literal"], m["replacement"], 0).encode("utf-8")
    row["sha_mutated"] = mv.digest(mutated)
    path.write_bytes(mutated)
try:
    r = subprocess.run(["cargo", "build", "--release", "--locked", "--offline", "--bin", "nexo"], cwd=tree, env=env,
                       capture_output=True, text=True)
    row["build_exit"] = r.returncode
    if r.returncode == 0:
        shutil.copyfile(Path(target) / "release" / "nexo.exe", out)
finally:
    if m is not None:
        path.write_bytes(original)
        row["sha_restored"] = mv.digest(path.read_bytes())
        assert row["sha_restored"] == row["sha_initial"]
print(json.dumps(row))
