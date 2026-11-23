"""verify_reg.py <cut> <id> <test_file_rel> <filter>: regression fails with the mutant and passes without it."""
import importlib.util, json, os, subprocess, sys, shutil
from pathlib import Path
cut, ident, tf, filt = sys.argv[1:5]
root = Path("C:/projects/Nexo-i07c-f"); tree = Path("C:/nexo-work/val/mut-regiones/tree")
spec = importlib.util.spec_from_file_location("mv", root / "scripts/mutant-verdict.py"); mv = importlib.util.module_from_spec(spec); spec.loader.exec_module(mv)
pop = json.loads((root / f"docs/validation/i07c-mut-regiones/population-{cut}.json").read_text(encoding="utf-8"))
m = next(x for x in pop["mutations"] if x["id"] == ident)
src = tree / m["file"]; orig = src.read_bytes(); testp = tree / tf; torig = testp.read_bytes()
env = dict(os.environ, CARGO_BUILD_JOBS="2")
def run():
    r = subprocess.run(["cargo", "nextest", "run", "--locked", "--offline", "--profile", "default", "--retries", "0", "-E", f"test({filt})"], cwd=tree, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return r.returncode, r.stderr[-600:]
try:
    shutil.copyfile(root / tf, testp)
    print("without mutant:", run())
    src.write_bytes(mv.replace_nth(orig.decode("utf-8"), m["literal"], m["replacement"], 0).encode("utf-8"))
    print("with mutant", ident, ":", run())
finally:
    src.write_bytes(orig); testp.write_bytes(torig)
    assert mv.digest(src.read_bytes()) == mv.digest(orig)
    print("restored", mv.digest(src.read_bytes())[:12])
