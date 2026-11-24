"""Parallel executor for the frozen mutation populations (one tanda).

Usage: run_par.py <cut> <count> <workers> [--root C:/nexo-work/tmp] [--max-seconds N]

Takes the next <count> pending ids of the population in the frozen shuffled order (field `order`),
deals them round-robin to <workers> workers. Every worker has its own `git archive HEAD` copy
(<root>/mutK/tree) and its own CARGO_TARGET_DIR (<root>/mutK/target) and runs `run_mut.py` (same
rules: full default suite, first failure = MUERTO, debug and release green = SUPERVIVIENTE,
1500 s timeout per profile, SHA-256 initial/mutated/restored per mutant) writing a shard file.
After all workers finish, the shards are appended to results-<cut>.jsonl (single writer) and the copies
and targets are deleted. Existing results are never rewritten. Run under scripts/heavy-lock.py.
"""
import hashlib, json, os, shutil, subprocess, sys, tarfile, io
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parents[2]
cut, count, workers = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
root = Path(sys.argv[sys.argv.index("--root") + 1]) if "--root" in sys.argv else Path("C:/nexo-work/tmp")
max_seconds = sys.argv[sys.argv.index("--max-seconds") + 1] if "--max-seconds" in sys.argv else "2400"
pop_path = here / f"population-{cut}.json"
results = here / f"results-{cut}.jsonl"
pop = json.loads(pop_path.read_text(encoding="utf-8"))
done = {json.loads(l)["id"] for l in results.read_text(encoding="utf-8").splitlines() if l.strip()} if results.exists() else set()
todo = [m["id"] for m in sorted(pop["mutations"], key=lambda m: m["order"]) if m["id"] not in done][:count]
if not todo:
    print("nothing pending"); sys.exit(0)
free_gb = shutil.disk_usage("C:/").free / 2**30
print(f"{cut}: {len(todo)} mutants, {workers} workers, free {free_gb:.1f} GB", flush=True)
assert free_gb >= 10 + 4 * workers, "not enough free disk"
base = {f: h for f, h in pop["blob_sha256"].items()}
procs = []
for k in range(workers):
    ids = todo[k::workers]
    if not ids:
        continue
    wroot = root / f"mut{k}"
    tree, target = wroot / "tree", wroot / "target"
    if wroot.exists():
        shutil.rmtree(wroot, ignore_errors=True)
    tree.mkdir(parents=True)
    data = subprocess.run(["git", "archive", "HEAD"], cwd=repo, capture_output=True, check=True).stdout
    tarfile.open(fileobj=io.BytesIO(data)).extractall(tree)
    for f, h in base.items():
        assert hashlib.sha256((tree / f).read_bytes()).hexdigest() == h, f"blob mismatch {f}"
    shard = wroot / "results.jsonl"
    env = dict(os.environ, CARGO_TARGET_DIR=str(target).replace("\\", "/"), CARGO_BUILD_JOBS="2",
               TEMP=str(wroot / "tmp"), TMP=str(wroot / "tmp"))
    (wroot / "tmp").mkdir()
    cmd = [sys.executable, "-B", str(here / "run_mut.py"), str(tree), str(pop_path), str(shard),
           "--max-seconds", max_seconds, "--ids", ",".join(ids)]
    procs.append((k, ids, shard, subprocess.Popen(cmd, env=env, stdout=open(wroot / "out.log", "w"), stderr=subprocess.STDOUT)))
rows = []
for k, ids, shard, p in procs:
    p.wait()
    if shard.exists():
        rows += [json.loads(l) for l in shard.read_text(encoding="utf-8").splitlines() if l.strip()]
order = {m["id"]: m["order"] for m in pop["mutations"]}
rows.sort(key=lambda r: order[r["id"]])
with results.open("a", encoding="utf-8", newline="\n") as fh:
    for r in rows:
        fh.write(json.dumps(r, ensure_ascii=False) + "\n")
for k, *_ in procs:
    shutil.rmtree(root / f"mut{k}", ignore_errors=True)
from collections import Counter
print(cut, "tanda:", len(rows), "of", len(todo), dict(Counter(r["verdict"] for r in rows)), flush=True)
