#!/usr/bin/env python3
"""Ronda 5, punto (1): outcome de check de main y candidato en los 28 (2.o brazo) y los 157 (propios del match), perfiles f0-0003 y f0-0005,
y `run` (exit y salida) para los aceptados. Uso: verif5.py MAIN_EXE CAND_EXE [MAIN2_EXE]"""
import sys, os, json, subprocess, tempfile, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(here, '..', '..', '..'))
MAIN, CAND = sys.argv[1], sys.argv[2]
B4 = os.path.join(ROOT, '.scratch', 'ref', 'b-d4')
c2 = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
def src(corp, i):
    if corp == 'c2': return c2[i]['wrapped']
    return json.load(open(os.path.join(B4, 'corp5-%s.json' % corp[1:])))[i]['w']
arm = json.load(open(os.path.join(here, 'armdiff-last.json')))
prop = json.load(open(os.path.join(here, 'matchdiff-last.json')))
S28 = [(a[0], a[1], 'anadido' if len(a[3]) > len(a[2]) else 'quitado') for a in arm]
S157 = []
for k, v in prop.items():
    corp = k.split('/')[0]
    for e in v: S157.append((corp, e[0]))
def run(exe, s, perfil, cmd):
    with tempfile.TemporaryDirectory() as td:
        f = os.path.join(td, 'p.n'); open(f, 'w', newline='').write(s)
        p = subprocess.run([exe, cmd, '--format', 'json', '--profile', perfil, f], capture_output=True, text=True, timeout=120)
    try: j = json.loads(p.stdout)
    except Exception: return ('ERR', p.returncode, '')
    return ((j.get('outcome') or {}).get('kind'), p.returncode, json.dumps({k: j.get(k) for k in ('stdout', 'events', 'observations')}, sort_keys=True), len(j.get('diagnostics', [])))
PROFS = ['f0-0003-b-v1', 'f0-0005-b-v1']
def cmp_set(name, items):
    rows = []
    def w(it):
        s = src(it[0], it[1]); out = {}
        for pr in PROFS:
            m, c = run(MAIN, s, pr, 'check'), run(CAND, s, pr, 'check')
            out[pr] = (m[0], c[0])
            if m[0] == 'checked' and c[0] == 'checked':
                rm, rc = run(MAIN, s, pr, 'run'), run(CAND, s, pr, 'run')
                out[pr] += (rm[:3] == rc[:3],)
        return it, out
    with ThreadPoolExecutor(6) as ex: rows = list(ex.map(w, items))
    cnt = collections.Counter(); bad = []
    for it, out in rows:
        for pr, v in out.items():
            cnt[(pr, v[0], v[1])] += 1
            if len(v) == 3 and not v[2]: bad.append((it, pr, 'run difiere'))
            if v[0] == 'checked' and v[1] == 'rejected': bad.append((it, pr, 'ACEPTADO->RECHAZADO'))
    print('==', name, len(items))
    for k, v in sorted(cnt.items()): print('  ', k, v)
    print('  problemas:', bad[:20] if bad else 'ninguno')
    return rows
r28 = cmp_set('28 segundo brazo', S28)
by = collections.defaultdict(list)
for (it, out) in r28: by[it[2]].append((it[:2], out))
for k, v in by.items():
    print(k, len(v), 'aceptados por main en algun perfil:', sum(1 for _, o in v if any(x[0] == 'checked' for x in o.values())))
cmp_set('157 propios del match', S157)
json.dump([(it, out) for it, out in r28], open(os.path.join(here, 'verif5-28.json'), 'w'))
