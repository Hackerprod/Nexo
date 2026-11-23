"""Runs a frozen mutation population one mutant at a time in a disposable copy (never the worktree).

Usage: run_mut.py <tree> <population.json> <results.jsonl> [--max-seconds N] [--ids M00001,...]
Order: field `order` of the population (seeded shuffle fixed before execution). Resumes: ids already
in results.jsonl are skipped. Per mutant: SHA-256 of the file checked before, after mutation and after
restoring; `cargo nextest run --profile default` (retries 0, fail-fast): debug first; MUERTO = first
failing test (named); compile error = NO COMPILA; debug green -> release; both green = SUPERVIVIENTE
(full default suite in both profiles). Reuses scripts/mutant-verdict.py (command, classify, replace_nth).
"""
import importlib.util, json, os, sys, time
from pathlib import Path

tree, pop_path, results = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), Path(sys.argv[3])
max_seconds = int(sys.argv[sys.argv.index("--max-seconds") + 1]) if "--max-seconds" in sys.argv else 10**9
only = set(sys.argv[sys.argv.index("--ids") + 1].split(",")) if "--ids" in sys.argv else None
spec = importlib.util.spec_from_file_location("mv", Path(__file__).resolve().parents[3] / "scripts/mutant-verdict.py")
mv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mv)
pop = json.loads(pop_path.read_text(encoding="utf-8"))
done = set()
if results.exists():
    done = {json.loads(l)["id"] for l in results.read_text(encoding="utf-8").splitlines() if l.strip()}
todo = sorted((m for m in pop["mutations"] if m["id"] not in done and (only is None or m["id"] in only)),
              key=lambda m: m["order"])
env = os.environ.copy()
env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
start = time.monotonic()
for m in todo:
    if time.monotonic() - start > max_seconds:
        break
    path = tree / m["file"]
    original = path.read_bytes()
    row = {"id": m["id"], "file": m["file"], "line": m["line"], "operator": m["operator"],
           "sha_initial": mv.digest(original), "profiles": []}
    t0 = time.monotonic()
    try:
        mutated = mv.replace_nth(original.decode("utf-8"), m["literal"], m["replacement"], 0).encode("utf-8")
        row["sha_mutated"] = mv.digest(mutated)
        path.write_bytes(mutated)
        for profile in ("debug", "release"):
            argv = ["cargo", "nextest", "run", "--locked", "--offline", "--profile", "default",
                    "--retries", "0", "--message-format", "libtest-json"]
            if profile == "release":
                argv.append("--release")
            result = mv.command(argv, str(tree), env, 1500)
            verdict, test, reason = mv.classify(result)
            row["profiles"].append({"profile": profile, "verdict": verdict, "test": test, "reason": reason,
                                    "seconds": result["seconds"]})
            if verdict == "VERDE":
                continue
            row["verdict"] = {"MUERTO": "MUERTO", "INVÁLIDO": "NO COMPILA"}.get(verdict, "INCONCLUSO")
            break
        else:
            row["verdict"] = "SUPERVIVIENTE"
    finally:
        path.write_bytes(original)
        row["sha_restored"] = mv.digest(path.read_bytes())
    row["seconds"] = round(time.monotonic() - t0, 1)
    assert row["sha_restored"] == row["sha_initial"], "restore failed"
    with results.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(m["id"], row["verdict"], row["profiles"][-1].get("test"), row["seconds"], flush=True)
