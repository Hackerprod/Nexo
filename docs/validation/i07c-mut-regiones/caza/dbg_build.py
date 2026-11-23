"""dbg_build.py <cut> <id|BASE> <out_exe>: builds the debug nexo.exe of a mutant in the mutation tree (restored afterwards)."""
import importlib.util, json, os, shutil, subprocess, sys
from pathlib import Path
cut, ident, out = sys.argv[1:4]
root = Path("C:/projects/Nexo-i07c-f"); tree = Path("C:/nexo-work/val/mut-regiones/tree")
spec = importlib.util.spec_from_file_location("mv", root / "scripts/mutant-verdict.py"); mv = importlib.util.module_from_spec(spec); spec.loader.exec_module(mv)
env = dict(os.environ, CARGO_BUILD_JOBS="2")
m = None
if ident != "BASE":
    m = next(x for x in json.loads((root / f"docs/validation/i07c-mut-regiones/population-{cut}.json").read_text(encoding="utf-8"))["mutations"] if x["id"] == ident)
    src = tree / m["file"]; orig = src.read_bytes()
    src.write_bytes(mv.replace_nth(orig.decode("utf-8"), m["literal"], m["replacement"], 0).encode("utf-8"))
try:
    r = subprocess.run(["cargo", "build", "--locked", "--offline", "--bin", "nexo"], cwd=tree, env=env, capture_output=True, text=True)
    print("build", r.returncode)
    if r.returncode == 0: shutil.copyfile("C:/nexo-work/target-mut-regiones/debug/nexo.exe", out)
finally:
    if m is not None:
        src.write_bytes(orig); assert mv.digest(src.read_bytes()) == mv.digest(orig); print("restored")
