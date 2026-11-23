#!/usr/bin/env python3
"""`run` de main (base-release.exe y main reconstruido) y candidato sobre TODOS los match/fn+match/matchexpr aceptados por main (f0-0003 y f0-0005)."""
import sys, os, json, subprocess, tempfile, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.abspath(os.path.join(here, '..', '..', '..'))
M1, M2, C = sys.argv[1:4]
B4 = os.path.join(ROOT, '.scratch', 'ref', 'b-d4')
progs = [e['wrapped'] for e in json.load(open(os.path.join(here, 'corpus2-congelado.json'))) if e['form'] in ('match', 'fn+match', 'matchexpr')]
for sd in ('7021993', '4417061', '9908255'):
    progs += [e['w'] for e in json.load(open(os.path.join(B4, 'corp5-%s.json' % sd))) if e['form'] in ('match', 'matchexpr')]
def run(exe, s, pr, cmd):
    with tempfile.TemporaryDirectory() as td:
        f = os.path.join(td, 'p.n'); open(f, 'w', newline='').write(s)
        p = subprocess.run([exe, cmd, '--format', 'json', '--profile', pr, f], capture_output=True, text=True, timeout=120)
    try: j = json.loads(p.stdout)
    except Exception: return ('ERR', p.returncode, '')
    return ((j.get('outcome') or {}).get('kind'), p.returncode, json.dumps({k: j.get(k) for k in ('stdout', 'events', 'observations', 'outcome')}, sort_keys=True))
for pr in ('f0-0003-b-v1', 'f0-0005-b-v1'):
    def w(s):
        m = run(M1, s, pr, 'check'); m2 = run(M2, s, pr, 'check'); c = run(C, s, pr, 'check')
        r = None
        if m[0] == 'checked':
            r = (run(M1, s, pr, 'run'), run(M2, s, pr, 'run'), run(C, s, pr, 'run'))
        return m[0], m2[0], c[0], r
    with ThreadPoolExecutor(6) as ex: res = list(ex.map(w, progs))
    acc = [r for r in res if r[0] == 'checked']
    print(pr, 'programas', len(progs), '| main acepta', len(acc), '| main(base) != main(reconstruido) en outcome:', sum(1 for r in res if r[0] != r[1]),
          '| cand acepta lo que main rechaza:', sum(1 for r in res if r[0] != 'checked' and r[2] == 'checked'),
          '| main acepta y cand rechaza:', sum(1 for r in res if r[0] == 'checked' and r[2] != 'checked'),
          '| run: main!=cand (exit/salida):', sum(1 for r in acc if r[3][0][:3] != r[3][2][:3]), '| main base!=reconstruido:', sum(1 for r in acc if r[3][0][:3] != r[3][1][:3]))
