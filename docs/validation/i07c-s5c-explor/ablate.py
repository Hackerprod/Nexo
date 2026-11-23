#!/usr/bin/env python3
"""Ablacion por pieza: ejecuta eval3 con el conjunto de interruptores menos cada pieza y resume
(regresiones core A frente a main, diferencias de firma completa frente al conjunto final, primer ejemplo).
Uso: ablate.py EXE FLAGS_FINALES TAG_FINAL pieza1,pieza2,... [--only c1,b,perm,t]"""
import sys, os, json, subprocess
here = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(here, '..', '..', '..', '.scratch', 'res3')
exe, flags, final, pieces = sys.argv[1], sys.argv[2].split(','), sys.argv[3], sys.argv[4].split(',')
only = sys.argv[sys.argv.index('--only') + 1] if '--only' in sys.argv else 'c1,b,perm,t'
fin = json.load(open(os.path.join(RES, final + '.json')))
out = {}
for p in pieces:
    tag = 'ABL_' + p
    fl = [f for f in flags if f != p]
    if not os.path.exists(os.path.join(RES, tag + '.json')):
        subprocess.run([sys.executable, '-B', os.path.join(here, 'eval3.py'), 'run', tag, '%s|%s' % (exe, ','.join(fl)), '--only', only], check=True, stdout=subprocess.DEVNULL)
    r = json.load(open(os.path.join(RES, tag + '.json')))
    t = subprocess.run([sys.executable, '-B', os.path.join(here, 'eval3.py'), 'table', 'main', tag], capture_output=True, text=True).stdout
    areg = [l for l in t.splitlines() if l.startswith('REGRESIONES')]
    diff = {k: [i for i in range(len(v)) if v[i] != fin[k][i]] for k, v in r.items() if k in fin}
    out[p] = {'regresiones': areg[0] if areg else '', 'difieren_de_final': {k: len(v) for k, v in diff.items()}, 'ejemplos': {k: v[:3] for k, v in diff.items() if v}}
    print(p, json.dumps(out[p], ensure_ascii=False)[:600], flush=True)
json.dump(out, open(os.path.join(here, 'ablacion-%s.json' % final), 'w'), indent=1)
