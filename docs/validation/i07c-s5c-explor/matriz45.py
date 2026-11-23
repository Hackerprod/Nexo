#!/usr/bin/env python3
"""Las 45 filas de la matriz: oraculo vs binarios (exploracion). Uso: matriz45.py NOMBRE=EXE[|FLAGS] ..."""
import sys, os, json
here = os.path.dirname(os.path.abspath(__file__))
ref = os.path.join(here, '..', '..', '..', '.scratch', 'ref')
sys.path.insert(0, os.path.join(ref, 'scripts')); sys.path.insert(0, here)
import i07c_s5c_oraculo as O
O.ROOT = ref
src = open(os.path.join(here, 'run_family.py')).read().replace('\nmain()\n', '\n')
ns = {'__file__': os.path.join(here, 'run_family.py')}; exec(compile(src, 'rf', 'exec'), ns)
S = O.load_sources()
rows = [r for r in O.ROWS]
bins = [a.split('=', 1) for a in sys.argv[1:]]
for name, exe in bins:
    xv = None
    if '|' in exe: exe, xv = exe.split('|', 1)
    ok_core = ok_full = 0; bad = []
    for r in rows:
        s = S[r]
        o = ns['orc'](s)
        b = ns['run_bin'](exe, s, 0, xv)
        if o is None: bad.append((r, 'oraculo-no-modelado')); continue
        c = lambda x: json.dumps([d[:4] for d in x]); f = lambda x: json.dumps(x)
        if c(b) == c(o): ok_core += 1
        else: bad.append((r, 'core'))
        if f(b) == f(o): ok_full += 1
        elif c(b) == c(o): bad.append((r, 'full-solo'))
    print(name, 'filas', len(rows), 'core OK', ok_core, 'full OK', ok_full, 'NO:', bad)
