#!/usr/bin/env python3
"""Ronda 8: clasifica por pieza las regresiones de core de un conjunto frente a main. Uso: cls8.py REGS.json EXE FLAGS"""
import sys, os, json
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
from concurrent.futures import ThreadPoolExecutor
regs = json.load(open(sys.argv[1])); exe = sys.argv[2]; flags = sys.argv[3].split(',')
main = json.load(open(os.path.join(E.RES, 'main.json')))
def srcs(name):
    if name == 'c1': return json.load(open(os.path.join(here, 'corpus-congelado.json')))
    if name == 't': return json.load(open(os.path.join(E.REF, 'corpus-t.json')))
    if name.startswith('s'): return [e['w'] for e in json.load(open(os.path.join(E.RES, '..', 'ref', 'b-d4', 'corp5-%s.json' % name[1:])))]
    raise SystemExit(name)
core = lambda r: json.dumps([x[:4] for x in r])
items = []
for k, idxs in regs.items():
    name, prof, _ = k.split('/'); S = srcs(name)
    for i in idxs: items.append((name, prof, i, S[i], main['%s|%s' % (name, prof)][i]))
def per(p):
    fl = ','.join(f for f in flags if f != p)
    with ThreadPoolExecutor(6) as ex:
        return list(ex.map(lambda it: E.run_bin(exe, it[3], it[1], fl), items))
res = {p: per(p) for p in flags}
cnt = {p: sum(1 for j, it in enumerate(items) if core(res[p][j]) == core(it[4])) for p in flags}
print('regresiones', len(items)); print('restituyen main al quitar la pieza:', cnt)
none = [j for j, it in enumerate(items) if not any(core(res[p][j]) == core(it[4]) for p in flags)]
print('ninguna pieza sola las restituye:', len(none))
by = {}
for j, it in enumerate(items):
    ps = tuple(p for p in flags if core(res[p][j]) == core(it[4]))
    by.setdefault(ps, []).append(j)
out = {', '.join(k) or 'ninguna': [(items[j][0], items[j][2]) for j in v] for k, v in by.items()}
for k, v in out.items(): print(k, len(v), v[:6])
json.dump({'items': [(it[0], it[1], it[2], it[3]) for it in items], 'por_pieza': out}, open(os.path.join(here, 'clasificacion-56.json'), 'w'))
