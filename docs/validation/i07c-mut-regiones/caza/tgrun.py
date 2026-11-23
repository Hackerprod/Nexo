import hashlib, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
d, base, mut, workers = Path(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4])
PROFILES = ["f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1"]
def run(exe, f, p):
    r = subprocess.run([exe, "check", f.name, "--profile", p, "--format", "json"], cwd=d, capture_output=True, timeout=60)
    return r.returncode, r.stdout
def one(f):
    diffs = []
    for p in PROFILES:
        if run(base, f, p) != run(mut, f, p): diffs.append(p)
    return f.name, diffs
with ThreadPoolExecutor(workers) as ex:
    res = list(ex.map(one, sorted(d.glob("*.n"))))
bad = [r for r in res if r[1]]
print(len(res), "programs;", len(bad), "differ", bad[:5])
