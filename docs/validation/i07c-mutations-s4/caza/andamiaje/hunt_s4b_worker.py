"""Run one of the two isolated S4 discriminator workers on the frozen corpus."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

import hunt_s4 as h

ROOT = Path(__file__).resolve().parent
SHA = 'd5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6'
BASE = Path('C:/nexo-work/cache/368f73fee16d6aea5b2db62d3e71ca0d00c85eba/585154a1ef365515/release.exe')
PROFILES = ['f0-0005-b-v1', 'f0-0003-b-v1', 'f0-0002-b-v2']


def sha(data):
    return hashlib.sha256(data).hexdigest()


def build(worker, mid, tree, target):
    env = os.environ.copy()
    env['PATH'] = ('C:/Users/hackerprod/.rustup/toolchains/1.99.1-x86_64-pc-windows-msvc/bin;'
                   'C:/Users/hackerprod/.cargo/bin;' + env.get('PATH', ''))
    env.update({'CARGO_TARGET_DIR': str(target), 'CARGO_PROFILE_DEV_DEBUG': '0',
                'CARGO_PROFILE_RELEASE_DEBUG': '0', 'CARGO_BUILD_JOBS': '4',
                'TEMP': 'C:/nexo-work/tmp', 'TMP': 'C:/nexo-work/tmp'})
    command = ['python', '-B', str(tree / 'scripts/heavy-lock.py'), 'run',
               '--session', f'i07c-s4-hunt-w{worker}', '--', 'sh', '-c',
               'export CARGO_BUILD_JOBS=4; cargo build --release --locked --offline --bin nexo']
    started = time.monotonic()
    while True:
        cp = subprocess.run(command, cwd=tree, env=env, capture_output=True,
                            text=True, encoding='utf-8', errors='replace')
        if cp.returncode == 75:
            print(f'{mid}: heavy lock occupied; waiting', flush=True)
            time.sleep(45)
            continue
        (ROOT / f'build-{mid}.log').write_text(cp.stdout + cp.stderr, encoding='utf-8')
        if cp.returncode:
            raise RuntimeError(f'{mid}: build failed ({cp.returncode}): {cp.stderr[-1000:]}')
        return round(time.monotonic() - started, 1)


def apply_exact(entry, source):
    before = source.read_bytes()
    assert sha(before) == SHA, (entry['id'], 'base SHA mismatch', sha(before))
    text = before.decode('utf-8')
    literal = entry['literal']
    assert text.count(literal) == 1, (entry['id'], 'literal not unique', text.count(literal))
    lines = text.splitlines(keepends=True)
    anchor = sum(len(x) for x in lines[:entry['line'] - 1])
    assert abs(text.find(literal) - anchor) <= 300, (entry['id'], 'line mismatch')
    source.write_bytes(text.replace(literal, entry['replacement'], 1).encode('utf-8'))
    return before


def main(worker):
    h.ROOT = ROOT
    h.TREE = Path(f'C:/nexo-work/val/hunt-s4b-w{worker}/tree')
    h.SRC = h.TREE / 'src/check/types.rs'
    h.TARGET = Path(f'C:/nexo-work/target-hunt-s4b-w{worker}')
    h.CORPUS_DIR = Path('C:/nexo-work/val/hunt-s4b/corpus')
    h.BASE = BASE
    assert BASE.exists(), BASE
    raw = (ROOT / 'corpus-frozen.json').read_bytes()
    freeze = json.loads((ROOT / 'corpus-freeze.json').read_text(encoding='utf-8'))
    assert sha(raw) == freeze['sha256']
    entries = json.loads(raw)
    assert len(entries) == freeze['entries']
    mutants = json.loads(Path('D:/nexo-work/mut-s3/s4-survivors-2.json').read_text(encoding='utf-8'))
    assigned = [m for i, m in enumerate(mutants) if i % 2 == worker]
    results = ROOT / f'hunt-resultados-w{worker}.jsonl'
    programs_file = ROOT / f'programas-discriminantes-w{worker}.json'
    done = set()
    if results.exists():
        done = {json.loads(line)['id'] for line in results.read_text(encoding='utf-8').splitlines() if line.strip()}
    programs = json.loads(programs_file.read_text(encoding='utf-8')) if programs_file.exists() else []
    base_cache = {}
    for entry in assigned:
        mid = entry['id']
        if mid in done:
            continue
        original = apply_exact(entry, h.SRC)
        try:
            build_seconds = build(worker, mid, h.TREE, h.TARGET)
            counts = dict.fromkeys(PROFILES, 0)
            differences = []
            started = time.monotonic()
            for profile in PROFILES:
                limit = len(entries) if profile == PROFILES[0] else 3040
                for i in range(limit):
                    key = (i, profile)
                    if key not in base_cache:
                        base_cache[key] = h.invoke(BASE, 'check', profile, i)
                    correct = base_cache[key]
                    mutant = h.invoke(h.TARGET / 'release/nexo.exe', 'check', profile, i)
                    observations = []
                    if correct['exit'] != mutant['exit'] or correct['json'] != mutant['json']:
                        observations.append(('check', correct, mutant))
                    if correct['exit'] == 0 or mutant['exit'] == 0:
                        correct_run = h.invoke(BASE, 'run', profile, i)
                        mutant_run = h.invoke(h.TARGET / 'release/nexo.exe', 'run', profile, i)
                        if correct_run['exit'] != mutant_run['exit'] or correct_run['json'] != mutant_run['json']:
                            observations.append(('run', correct_run, mutant_run))
                    counts[profile] += 1
                    for mode, good, bad in observations:
                        item = {'id': mid, 'index': i, 'family': entries[i]['family'],
                                'source': entries[i]['source'], 'profile': profile, 'mode': mode,
                                'correct': good, 'mutant': bad}
                        differences.append(item)
                        programs.append(item)
                    if len(differences) >= 5:
                        break
                    if counts[profile] % 1200 == 0:
                        print(f'w{worker} {mid} {profile}: {counts[profile]}/{limit}; differences {len(differences)}', flush=True)
                if len(differences) >= 5:
                    break
            row = {'id': mid, 'worker': worker,
                   'status': 'DISCRIMINADO' if differences else 'SIN_DIFERENCIA',
                   'counts': counts, 'total_comparisons': sum(counts.values()),
                   'differences': len(differences), 'first_indices': [d['index'] for d in differences[:5]],
                   'build_seconds': build_seconds,
                   'hunt_seconds': round(time.monotonic() - started, 1),
                   'corpus_sha256': freeze['sha256'], 'seed': freeze['seed']}
            with results.open('a', encoding='utf-8') as output:
                output.write(json.dumps(row, ensure_ascii=False) + '\n')
            programs_file.write_text(json.dumps(programs, ensure_ascii=False, indent=2), encoding='utf-8')
            print(json.dumps(row, ensure_ascii=False), flush=True)
        finally:
            h.SRC.write_bytes(original)
            assert sha(h.SRC.read_bytes()) == SHA, (mid, 'restore SHA mismatch')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker', type=int, choices=(0, 1), required=True)
    main(parser.parse_args().worker)
