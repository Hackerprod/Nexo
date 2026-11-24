"""Parallel executor for the frozen mutation populations (one tanda).

Usage: run_par.py <cut> <count> <workers> [--root C:/nexo-work/tmp] [--max-seconds N]

Takes the next <count> pending ids of the population in the frozen shuffled order (field `order`),
deals them round-robin to <workers> workers. Every worker has its own `git archive HEAD` copy
(<root>/mutK/tree) and its own CARGO_TARGET_DIR (<root>/mutK/target) and runs `run_mut.py` (same
rules: full default suite, first failure = MUERTO, debug and release green = SUPERVIVIENTE,
1500 s timeout per profile, SHA-256 initial/mutated/restored per mutant) writing a shard file.

Resource protection (memguard.py; added after a mutant made tests allocate 26,9 + 22,0 + 6,4 GB of commit
and exhausted the Windows commit limit):
- before the tanda: FreeVirtualMemory > 25 GB (else waits 150 s, up to 20 min);
- every worker lives in a Job Object with kernel-enforced commit limits (8 GB per process, 16 GB per worker);
- every 10 s: a process of the worker tree above 6 GB of COMMIT (PrivatePageCount, compilers excepted), a
  worker tree above 10 GB, or FreeVirtualMemory < 15 GB -> the largest-commit process of OUR tree is killed
  by PID; the mutant in flight is recorded INCONCLUSO (reason `memoria`) to be repeated, and the event
  is written to results-<cut>-memoria.jsonl;
- a worker with no activity (out.log, shard, tmp, target dirs) for 20 min is aborted (`taskkill /PID <own pid> /T`).
A first INCONCLUSO of an id goes to results-<cut>-inconclusos.jsonl and the id stays pending (repeated in a later
tanda); a second one is kept in results-<cut>.jsonl. Existing results are never rewritten.
After all workers finish, shards are appended to results-<cut>.jsonl (single writer) and copies/targets are deleted.
Run under scripts/heavy-lock.py, tandas of <= 20 min with >= 150 s between them.
"""
import hashlib, io, json, os, shutil, subprocess, sys, tarfile, time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import memguard

here = Path(__file__).resolve().parent
repo = here.parents[2]
cut, count, workers = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
root = Path(sys.argv[sys.argv.index("--root") + 1]) if "--root" in sys.argv else Path("C:/nexo-work/tmp")
max_seconds = sys.argv[sys.argv.index("--max-seconds") + 1] if "--max-seconds" in sys.argv else "2400"
STALL = int(os.environ.get("RUN_PAR_STALL_SECONDS", "1200"))
pop_path = here / f"population-{cut}.json"
results = here / f"results-{cut}.jsonl"
incon = here / f"results-{cut}-inconclusos.jsonl"
memlog = here / f"results-{cut}-memoria.jsonl"
pop = json.loads(pop_path.read_text(encoding="utf-8"))


def ids_of(path):
    if not path.exists():
        return set()
    return {json.loads(l)["id"] for l in path.read_text(encoding="utf-8").splitlines() if l.strip()}


done = ids_of(results)
todo = [m["id"] for m in sorted(pop["mutations"], key=lambda m: m["order"]) if m["id"] not in done][:count]
if not todo:
    print("nothing pending")
    sys.exit(0)
waited = 0
while memguard.free_virtual_gb() < 25:
    print("FreeVirtualMemory < 25 GB: esperando 150 s", flush=True)
    time.sleep(150)
    waited += 1
    if waited > 8:
        print("memoria virtual insuficiente tras 20 min: no se lanza la tanda", flush=True)
        sys.exit(3)
free_gb = shutil.disk_usage("C:/").free / 2**30
print(f"{cut}: {len(todo)} mutants, {workers} workers, free disk {free_gb:.1f} GB, free virtual {memguard.free_virtual_gb():.1f} GB", flush=True)
assert free_gb >= 12 + 2 * workers, "not enough free disk"
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
    (wroot / "tmp").mkdir()
    data = subprocess.run(["git", "archive", "HEAD"], cwd=repo, capture_output=True, check=True).stdout
    tarfile.open(fileobj=io.BytesIO(data)).extractall(tree, filter="data")
    for f, h in pop["blob_sha256"].items():
        assert hashlib.sha256((tree / f).read_bytes()).hexdigest() == h, f"blob mismatch {f}"
    shard = wroot / "results.jsonl"
    env = dict(os.environ, CARGO_TARGET_DIR=str(target).replace("\\", "/"), CARGO_BUILD_JOBS="4",
               NEXTEST_TEST_THREADS="4", TEMP=str(wroot / "tmp"), TMP=str(wroot / "tmp"))
    cmd = [sys.executable, "-B", str(here / "run_mut.py"), str(tree), str(pop_path), str(shard),
           "--max-seconds", max_seconds, "--ids", ",".join(ids)]
    p = subprocess.Popen(cmd, env=env, stdout=open(wroot / "out.log", "w"), stderr=subprocess.STDOUT)
    job = memguard.limit_job(p, 8, 16)
    procs.append((k, ids, shard, p, job))


def last_activity(k):
    best = 0.0
    w = root / f"mut{k}"
    for q in (w / "out.log", w / "results.jsonl", w / "tmp", w / "target" / "debug", w / "target" / "release",
              w / "target" / "debug" / "deps", w / "target" / "release" / "deps"):
        try:
            best = max(best, q.stat().st_mtime)
        except OSError:
            pass
    return best


def inflight(ids, shard):
    got = set()
    if shard.exists():
        got = {json.loads(l)["id"] for l in shard.read_text(encoding="utf-8").splitlines() if l.strip()}
    return next((i for i in ids if i not in got), None)


aborted = {}
memory_hits = {}  # id -> reason
started = time.time()
while any(p.poll() is None for _, _, _, p, _ in procs):
    time.sleep(10)
    live = [(k, ids, shard, p) for k, ids, shard, p, _ in procs if p.poll() is None]
    for pid, why, gb in memguard.check([p.pid for *_, p in live], proc_gb=6, tree_gb=10, min_free_gb=15):
        owner = next((x for x in live if pid in {r["ProcessId"] for r in memguard.tree(x[3].pid, memguard.process_table())}), None)
        mid = inflight(owner[1], owner[2]) if owner else None
        memguard.kill_pid(pid)
        memory_hits[mid] = why
        with memlog.open("a", encoding="utf-8", newline="\n") as fh:
            fh.write(json.dumps({"id": mid, "pid": pid, "reason": why, "commit_gb": round(gb, 1),
                                 "time": time.strftime("%Y-%m-%d %H:%M:%S")}) + "\n")
        print(f"memoria: mutante {mid}: proceso {pid} muerto ({why})", flush=True)
    for k, ids, shard, p in live:
        if k not in aborted and time.time() - max(last_activity(k), started) > STALL:
            aborted[k] = "sin progreso"
            subprocess.run(["taskkill", "/PID", str(p.pid), "/T", "/F"], capture_output=True)  # our own worker tree
            print(f"worker {k} abortado (sin progreso)", flush=True)
rows = []
for k, ids, shard, p, job in procs:
    p.wait()
    got = set()
    if shard.exists():
        part = [json.loads(l) for l in shard.read_text(encoding="utf-8").splitlines() if l.strip()]
        rows += part
        got = {r["id"] for r in part}
    if k in aborted:
        for i in [i for i in ids if i not in got][:1]:
            memory_hits.setdefault(i, f"watchdog: {aborted[k]}")
by_id = {r["id"]: r for r in rows}
for mid, why in memory_hits.items():
    if mid is None:
        continue
    m = next(x for x in pop["mutations"] if x["id"] == mid)
    old = by_id.get(mid)
    rows = [r for r in rows if r["id"] != mid]
    rows.append({"id": mid, "file": m["file"], "line": m["line"], "operator": m["operator"],
                 "profiles": old["profiles"] if old else [], "verdict": "INCONCLUSO",
                 "reason": f"memoria: {why}", "seconds": old["seconds"] if old else 0,
                 "sha_initial": pop["blob_sha256"][m["file"]], "sha_restored": pop["blob_sha256"][m["file"]]})
order = {m["id"]: m["order"] for m in pop["mutations"]}
rows.sort(key=lambda r: order[r["id"]])
before = ids_of(incon)
keep = []
for r in rows:
    if r["verdict"] == "INCONCLUSO" and r["id"] not in before:
        with incon.open("a", encoding="utf-8", newline="\n") as fh:
            fh.write(json.dumps(r, ensure_ascii=False) + "\n")
    else:
        keep.append(r)
with results.open("a", encoding="utf-8", newline="\n") as fh:
    for r in keep:
        fh.write(json.dumps(r, ensure_ascii=False) + "\n")
for k, *_ in procs:
    shutil.rmtree(root / f"mut{k}", ignore_errors=True)
print(cut, "tanda:", len(rows), "of", len(todo), dict(Counter(r["verdict"] for r in rows)), flush=True)
