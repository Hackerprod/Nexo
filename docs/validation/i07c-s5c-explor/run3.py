#!/usr/bin/env python3
"""Submuestra de `run` (debug/release, main/HF) y pasadas de planning. Uso: run3.py FLAGS"""
import sys, json, os, re, subprocess, threading, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__))
B = 'C:/projects/Nexo-i07c-d/.scratch/bin/'
FL = sys.argv[1]
W = os.path.join(os.environ.get('TEMP', '.'), 'run3'); os.makedirs(W, exist_ok=True)
REF = os.path.join(here, '..', '..', '..', '.scratch', 'ref', 'docs', 'validation', 'i07c-s5c', 'b-d2')
c2 = [e['wrapped'] for e in json.load(open(os.path.join(here, 'corpus2-congelado.json')))][::5]
b = json.load(open(os.path.join(REF, 'corpus-b.json')))[::2]
t = json.load(open(os.path.join(REF, 'corpus-t.json')))[::6]
progs = [(s, 'f0-0003-b-v1') for s in c2] + [(s, 'f0-0002-b-v2') for s in b + t]
def run(exe, s, prof, cmd, xv=None):
    f = os.path.join(W, 'f%d.n' % threading.get_ident()); open(f, 'w', newline='').write(s)
    p = subprocess.run([exe, cmd, '--format', 'json', '--profile', prof, f], capture_output=True, text=True, timeout=120, env=dict(os.environ, NEXO_XV=xv) if xv is not None else None)
    return re.sub(r'"file": ?"[^"]*"', '"file":""', p.stdout), p.returncode, p.stderr
def go(name, exe, xv):
    with ThreadPoolExecutor(6) as ex:
        return list(ex.map(lambda a: run(exe, a[0], a[1], 'run', xv)[:2], progs))
res = {'main_rel': go('m', B + 'main.exe', None), 'main_dbg': go('md', B + 'main-dbg.exe', None),
       'hf_rel': go('h', B + 'r8.exe', FL), 'hf_dbg': go('hd', B + 'r8-dbg.exe', FL)}
n = len(progs)
dm = [i for i in range(n) if res['main_rel'][i] != res['main_dbg'][i]]
dh = [i for i in range(n) if res['hf_rel'][i] != res['hf_dbg'][i]]
dmh = [i for i in range(n) if res['main_rel'][i] != res['hf_rel'][i]]
# diferencias main vs HF que no se explican por check
def chk(exe, s, prof, xv): return run(exe, s, prof, 'check', xv)[0]
unexpl = []
with ThreadPoolExecutor(6) as ex:
    cm = list(ex.map(lambda i: chk(B + 'main.exe', progs[i][0], progs[i][1], None), dmh))
    ch = list(ex.map(lambda i: chk(B + 'r8.exe', progs[i][0], progs[i][1], FL), dmh))
unexpl = [i for k, i in enumerate(dmh) if cm[k] == ch[k]]
print('programas run:', n, '| main debug!=release:', len(dm), '| HF debug!=release:', len(dh), '| main!=HF (run rel):', len(dmh), '| sin explicar por check:', len(unexpl), unexpl[:5])
# pasadas
def passes(s, prof):
    return [int(l.split()[1]) for l in run(B + 'r8.exe', s, prof, 'check', FL + ',passes')[2].splitlines() if l.startswith('PASSES')]
with ThreadPoolExecutor(6) as ex:
    ps = list(ex.map(lambda a: max(passes(a[0], a[1]) or [0]), progs))
print('pasadas maximas (muestra):', max(ps), sorted(collections.Counter(ps).items()))
