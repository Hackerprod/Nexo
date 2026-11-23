"""Runs one patch (inverse mutation or population member) in a disposable copy.

Usage: mut_one.py <tree> <patch.json> <id> <out.json> [--debug-only] [--only <nextest filterset>]
Reuses scripts/mutant-verdict.py (digest, command, classify, replace_nth) without its allowed-root
check (the copy lives under .scratch/ of the worktree). Selection: full default profile, retries 0,
fail-fast; debug first, release only if debug is green. MUERTO = first failing test (named);
SUPERVIVIENTE = both profiles green. The mutated file is restored byte by byte and its SHA-256
checked (sha_restored == sha_initial).
"""
import importlib.util, json, os, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("mv", root / "scripts/mutant-verdict.py")
mv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mv)

tree, patch_path, ident, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4])
debug_only = "--debug-only" in sys.argv
only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None  # targeted rerun of survivors against one test
patch = json.loads(patch_path.read_text(encoding="utf-8"))
src = tree / patch["file"]
original = src.read_bytes()
assert mv.digest(original) == patch["sha_initial"], "sha_initial"
mutated = mv.replace_nth(original.decode("utf-8"), patch["before"], patch["after"], patch.get("occurrence", 0)).encode("utf-8")
assert mv.digest(mutated) == patch["sha_mutated"], "sha_mutated"
env = os.environ.copy()
env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
row = {"id": ident, "file": patch["file"], "sha_initial": patch["sha_initial"], "sha_mutated": patch["sha_mutated"], "profiles": []}
src.write_bytes(mutated)
try:
    for profile in ("debug", "release"):
        argv = ["cargo", "nextest", "run", "--locked", "--offline", "--profile", "default", "--retries", "0", "--message-format", "libtest-json"]
        if profile == "release":
            argv.append("--release")
        if only:
            argv += ["-E", only]
        result = mv.command(argv, str(tree), env, 2400)
        verdict, test, reason = mv.classify(result)
        row["profiles"].append({"profile": profile, "verdict": verdict, "test": test, "reason": reason, "seconds": result["seconds"]})
        if verdict != "VERDE" or debug_only:
            row["verdict"] = verdict
            break
    else:
        row["verdict"] = "SUPERVIVIENTE"
finally:
    src.write_bytes(original)
    row["sha_restored"] = mv.digest(src.read_bytes())
assert row["sha_restored"] == patch["sha_initial"]
out.write_text(json.dumps(row, indent=1), encoding="utf-8")
print(json.dumps(row))
