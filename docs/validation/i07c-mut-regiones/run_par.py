"""Parallel executor for the frozen mutation populations (one tanda).

Usage: run_par.py <cut> <count> <workers> [--root C:/nexo-work/tmp] [--max-seconds N]

Takes the next <count> pending ids of the population in the frozen shuffled order (field `order`),
deals them round-robin to <workers> workers. Every worker has its own `git archive HEAD` copy
(<root>/mutK/tree) and its own CARGO_TARGET_DIR (<root>/mutK/target) and runs `run_mut.py` (same
rules: full default suite, first failure = MUERTO, debug and release green = SUPERVIVIENTE,
1500 s timeout per profile, SHA-256 initial/mutated/restored per mutant) writing a shard file.

Watchdog (added after a tanda hung with a MemoryError in a worker): every 30 s the process tree of
each worker is measured; a worker whose tree exceeds RUN_PAR_MEM_GB (default 10) of working set, or
whose copy shows no activity (out.log, shard, tmp dir) for RUN_PAR_STALL_SECONDS (default 1200), is
aborted with `taskkill /PID <its own pid> /T /F` and the mutant it was running is recorded
INCONCLUSO (reason `watchdog`). A first INCONCLUSO of an id goes to results-<cut>-inconclusos.jsonl and
the id stays pending (it is repeated in a later tanda); a second one is kept in results-<cut>.jsonl.

After all workers finish, the shards are appended to results-<cut>.jsonl (single writer) and the copies
and targets are deleted. Existing results are never rewritten. Run under scripts/heavy-lock.py.
"""
import hashlib, io, json, os, shutil, subprocess, sys, tarfile, time
from collections import Counter
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parents[2]
cut, count, workers = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
root = Path(sys.argv[sys.argv.index("--root") + 1]) if "--root" in sys.argv else Path("C:/nexo-work/tmp")
max_seconds = sys.argv[sys.argv.index("--max-seconds") + 1] if "--max-seconds" in sys.argv else "2400"
STALL = int(os.environ.get("RUN_PAR_STALL_SECONDS", "1200"))
MEM_LIMIT = int(os.environ.get("RUN_PAR_MEM_GB", "10")) * 2**30
pop_path = here / f"population-{cut}.json"
results = here / f"results-{cut}.jsonl"
incon = here / f"results-{cut}-inconclusos.jsonl"
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
free_gb = shutil.disk_usage("C:/").free / 2**30
print(f"{cut}: {len(todo)} mutants, {workers} workers, free {free_gb:.1f} GB", flush=True)
assert free_gb >= 10 + 3 * workers, "not enough free disk"
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
    env = dict(os.environ, CARGO_TARGET_DIR=str(target).replace("\\", "/"), CARGO_BUILD_JOBS="2",
               NEXTEST_TEST_THREADS=str(max(2, min(6, (os.cpu_count() or 24) // workers))),
               TEMP=str(wroot / "tmp"), TMP=str(wroot / "tmp"))
    cmd = [sys.executable, "-B", str(here / "run_mut.py"), str(tree), str(pop_path), str(shard),
           "--max-seconds", max_seconds, "--ids", ",".join(ids)]
    procs.append((k, ids, shard, subprocess.Popen(cmd, env=env, stdout=open(wroot / "out.log", "w"), stderr=subprocess.STDOUT)))


def process_table():
    out = subprocess.run(["powershell", "-NoProfile", "-Command",
                          "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,WorkingSetSize | ConvertTo-Json -Compress"],
                         capture_output=True, text=True).stdout
    try:
        return json.loads(out)
    except Exception:
        return []


def tree_ws(root_pid, table):
    kids = {}
    for r in table:
        kids.setdefault(r["ParentProcessId"], []).append(r["ProcessId"])
    ws = {r["ProcessId"]: r["WorkingSetSize"] or 0 for r in table}
    total, stack = 0, [root_pid]
    while stack:
        pid = stack.pop()
        total += ws.get(pid, 0)
        stack += kids.get(pid, [])
    return total


def last_activity(k):
    best = 0.0
    for q in (root / f"mut{k}" / "out.log", root / f"mut{k}" / "results.jsonl", root / f"mut{k}" / "tmp"):
        try:
            best = max(best, q.stat().st_mtime)
        except OSError:
            pass
    return best


aborted = {}
started = time.time()
while any(p.poll() is None for *_, p in procs):
    time.sleep(30)
    table = process_table()
    for k, ids, shard, p in procs:
        if p.poll() is not None or k in aborted:
            continue
        reason = None
        if tree_ws(p.pid, table) > MEM_LIMIT:
            reason = "memoria"
        elif time.time() - max(last_activity(k), started) > STALL:
            reason = "sin progreso"
        if reason:
            aborted[k] = reason
            subprocess.run(["taskkill", "/PID", str(p.pid), "/T", "/F"], capture_output=True)  # only our own tree
            print(f"worker {k} abortado ({reason})", flush=True)
rows = []
for k, ids, shard, p in procs:
    p.wait()
    got = set()
    if shard.exists():
        part = [json.loads(l) for l in shard.read_text(encoding="utf-8").splitlines() if l.strip()]
        rows += part
        got = {r["id"] for r in part}
    if k in aborted:
        for i in [i for i in ids if i not in got][:1]:
            m = next(x for x in pop["mutations"] if x["id"] == i)
            rows.append({"id": i, "file": m["file"], "line": m["line"], "operator": m["operator"], "profiles": [],
                         "verdict": "INCONCLUSO", "reason": f"watchdog: {aborted[k]}", "seconds": 0,
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
