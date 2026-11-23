#!/usr/bin/env python3
"""Cambios de aceptacion de `check` (checked/rejected) entre main y variantes sobre la familia dirigida (3 perfiles)."""
import sys, os, json, subprocess, tempfile, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.abspath(os.path.join(here, '..', '..', '..'))
B = os.path.join(ROOT, '.scratch', 'bin')
progs = json.load(open(os.path.join(here, 'corpus-dirigido5.json')))
D = 'pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr'
VARS = {'main': (os.path.join(B, 'main.exe'), None), 'cand4 (skip original)': (os.path.join(B, 'cand4.exe'), None),
        'skipv2': (os.path.join(B, 'sk.exe'), D + ',skipv2'), 'skipv3': (os.path.join(B, 'sk.exe'), D + ',skipv3')}
def run(exe, xv, s, pr):
    with tempfile.TemporaryDirectory() as td:
        f = os.path.join(td, 'p.n'); open(f, 'w', newline='').write(s)
        p = subprocess.run([exe, 'check', '--format', 'json', '--profile', pr, f], capture_output=True, text=True, timeout=60, env=dict(os.environ, NEXO_XV=xv) if xv is not None else None)
    try: j = json.loads(p.stdout)
    except Exception: return 'ERR'
    return (j.get('outcome') or {}).get('kind'), len(j.get('diagnostics', []))
for pr in ('f0-0002-b-v2', 'f0-0003-b-v1', 'f0-0005-b-v1'):
    res = {}
    for name, (exe, xv) in VARS.items():
        with ThreadPoolExecutor(6) as ex: res[name] = list(ex.map(lambda s: run(exe, xv, s, pr), progs))
    acc = sum(1 for r in res['main'] if r[0] == 'checked')
    print(pr, 'programas', len(progs), 'main acepta', acc)
    for name in list(VARS)[1:]:
        ch = [i for i in range(len(progs)) if res['main'][i][0] != res[name][i][0]]
        nd = sum(1 for i in range(len(progs)) if res['main'][i][1] != res[name][i][1])
        print('   %-24s cambios de aceptacion: %d | programas con distinto numero de diagnosticos: %d | ej: %s' % (name, len(ch), nd, [progs[i][:90] for i in ch[:2]]))
