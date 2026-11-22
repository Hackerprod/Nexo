"""Frozen-source discriminator hunt for the I07c S4 survivors."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

ROOT = Path(__file__).resolve().parent
TREE = Path('C:/nexo-work/val/hunt-s4/tree')
SRC = TREE / 'src/check/types.rs'
TARGET = Path('C:/nexo-work/target-hunt-s4')
CORPUS_DIR = Path('C:/nexo-work/val/hunt-s4/corpus')
BASE = Path('C:/nexo-work/cache/368f73fee16d6aea5b2db62d3e71ca0d00c85eba/585154a1ef365515/release.exe')
SHA = 'd5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6'
PROFILES = ['f0-0005-b-v1', 'f0-0003-b-v1', 'f0-0002-b-v2']

def sha(data):
    return hashlib.sha256(data).hexdigest()

def invoke(exe, mode, profile, index):
    cmd = [str(exe), mode, '--profile', profile, '--format', 'json', str(CORPUS_DIR / f'p{index:05d}.n')]
    for attempt in range(2):
        try:
            cp = subprocess.run(cmd, capture_output=True, timeout=20)
            raw = cp.stdout.decode('utf-8', 'replace')
            try:
                obj = json.loads(raw)
            except json.JSONDecodeError:
                obj = {'non_json_stdout': raw}
            return {'exit': cp.returncode, 'json': obj, 'stderr': cp.stderr.decode('utf-8', 'replace')}
        except subprocess.TimeoutExpired:
            with (ROOT / 'timeouts.jsonl').open('a', encoding='utf-8') as log:
                log.write(json.dumps({'exe': str(exe), 'mode': mode, 'profile': profile,
                                      'index': index, 'attempt': attempt + 1}) + '\n')
    return {'exit': None, 'json': {'timeout': True}, 'stderr': ''}

def build(mutant_id):
    env = os.environ.copy()
    env['PATH'] = ('C:/Users/hackerprod/.rustup/toolchains/1.99.1-x86_64-pc-windows-msvc/bin;'
                   'C:/Users/hackerprod/.cargo/bin;' + env.get('PATH', ''))
    env.update({'CARGO_TARGET_DIR': str(TARGET).replace('\\', '/'),
                'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_RELEASE_DEBUG': '0',
                'CARGO_BUILD_JOBS': '2', 'TEMP': 'C:/nexo-work/tmp', 'TMP': 'C:/nexo-work/tmp'})
    # The lock helper sets jobs=6; the inner shell restores the stricter task limit.
    cmd = ['python', '-B', str(TREE / 'scripts/heavy-lock.py'), 'run',
           '--session', 'i07c-s4-hunt', '--', 'sh', '-c',
           'export CARGO_BUILD_JOBS=2; cargo build --release --locked --offline --bin nexo']
    start = time.monotonic()
    while True:
        cp = subprocess.run(cmd, cwd=TREE, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace')
        if cp.returncode != 75:
            (ROOT / f'build-{mutant_id}.log').write_text(cp.stdout + cp.stderr, encoding='utf-8')
            if cp.returncode:
                raise RuntimeError(f'build {mutant_id} failed ({cp.returncode}): {cp.stderr[-1000:]}')
            return round(time.monotonic() - start, 1)
        print(f'{mutant_id}: heavy lock occupied; waiting', flush=True)
        time.sleep(45)

def apply_exact(m):
    data = SRC.read_bytes()
    assert sha(data) == SHA, f'base hash changed: {sha(data)}'
    src = data.decode('utf-8')
    literal = m['literal']
    replacement = m['replacement']
    assert src.count(literal) == 1, (m['id'], src.count(literal))
    lines = src.splitlines(keepends=True)
    start = sum(len(x) for x in lines[:m['line'] - 1])
    assert src.find(literal) >= start - 300 and src.find(literal) <= start + 300, m['id']
    SRC.write_bytes(src.replace(literal, replacement, 1).encode('utf-8'))
    return data

def main():
    freeze = json.loads((ROOT / 'corpus-freeze.json').read_text(encoding='utf-8'))
    raw = (ROOT / 'corpus-frozen.json').read_bytes()
    assert sha(raw) == freeze['sha256']
    entries = json.loads(raw)
    assert len(entries) == freeze['entries']
    assert BASE.exists() and sha(SRC.read_bytes()) == SHA
    mutants = json.loads(Path('D:/nexo-work/mut-s3/s4-survivors-1.json').read_text(encoding='utf-8'))
    priority = ['M00043', 'M00018', 'M00021', 'M00028', 'M00034', 'M00044',
                'M00008', 'M00009', 'M00010', 'M00011', 'M00025']
    mutants.sort(key=lambda m: priority.index(m['id']))
    done = set()
    results = ROOT / 'hunt-resultados.jsonl'
    if results.exists():
        done = {json.loads(line)['id'] for line in results.read_text(encoding='utf-8').splitlines() if line.strip()}
    discriminants = []
    if (ROOT / 'programas-discriminantes.json').exists():
        discriminants = json.loads((ROOT / 'programas-discriminantes.json').read_text(encoding='utf-8'))
    base_cache = {}

    def compare_one(task):
        i, profile = task
        key = (i, profile)
        base_check = base_cache.get(key)
        if base_check is None:
            base_check = invoke(BASE, 'check', profile, i)
            base_cache[key] = base_check
        mutant_check = invoke(TARGET / 'release/nexo.exe', 'check', profile, i)
        differences = []
        if base_check['exit'] != mutant_check['exit'] or base_check['json'] != mutant_check['json']:
            differences.append(('check', base_check, mutant_check))
        if base_check.get('exit') == 0 or mutant_check.get('exit') == 0:
            base_run = invoke(BASE, 'run', profile, i)
            mutant_run = invoke(TARGET / 'release/nexo.exe', 'run', profile, i)
            if base_run['exit'] != mutant_run['exit'] or base_run['json'] != mutant_run['json']:
                differences.append(('run', base_run, mutant_run))
        return i, profile, differences

    for m in mutants:
        mid = m['id']
        if mid in done:
            continue
        original = apply_exact(m)
        try:
            build_seconds = build(mid)
            counts = {p: 0 for p in PROFILES}
            diffs = []
            start_time = time.monotonic()
            # All families in the frozen order; the newer directed sources come first.
            with ThreadPoolExecutor(max_workers=3) as pool:
                for profile in PROFILES:
                    # The full frozen corpus runs in the profile exposing S4's
                    # defect; all three profiles run the directed 3,040 sources.
                    limit = len(entries) if profile == PROFILES[0] else 3040
                    for start in range(0, limit, 120):
                        tasks = ((i, profile) for i in range(start, min(start + 120, limit)))
                        for i, p, observations in pool.map(compare_one, tasks):
                            counts[p] += 1
                            for mode, correct, mutant in observations:
                                item = {'id': mid, 'index': i, 'family': entries[i]['family'],
                                        'source': entries[i]['source'], 'profile': p, 'mode': mode,
                                        'correct': correct, 'mutant': mutant}
                                diffs.append(item)
                                discriminants.append(item)
                            if len(diffs) >= 5:
                                break
                        if len(diffs) >= 5:
                            break
                        if counts[profile] % 1200 == 0:
                            print(f'{mid} {profile}: {counts[profile]}/{limit}; differences {len(diffs)}', flush=True)
                    if len(diffs) >= 5:
                        break
            result = {'id': mid, 'status': 'DISCRIMINADO' if diffs else 'SIN_DIFERENCIA',
                      'counts': counts, 'total_comparisons': sum(counts.values()),
                      'differences': len(diffs), 'first_indices': [d['index'] for d in diffs[:5]],
                      'build_seconds': build_seconds,
                      'hunt_seconds': round(time.monotonic() - start_time, 1),
                      'corpus_sha256': freeze['sha256'], 'seed': freeze['seed']}
            with results.open('a', encoding='utf-8') as f:
                f.write(json.dumps(result, ensure_ascii=False) + '\n')
            (ROOT / 'programas-discriminantes.json').write_text(
                json.dumps(discriminants, ensure_ascii=False, indent=2), encoding='utf-8')
            print(json.dumps(result, ensure_ascii=False), flush=True)
        finally:
            SRC.write_bytes(original)
            assert sha(SRC.read_bytes()) == SHA, f'{mid}: source not restored'

if __name__ == '__main__':
    main()
