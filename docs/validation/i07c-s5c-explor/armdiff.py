#!/usr/bin/env python3
"""Diferencia main vs candidato en los diagnosticos cuyo primario cae en el SEGUNDO brazo de un `match` como expresion
(la senal propia de la union de brazos). Uso: armdiff.py EXE_MAIN EXE_CAND FLAGS"""
import sys, os, json, re, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
M, C, FL = sys.argv[1:4]
P3 = 'f0-0003-b-v1'
items = []
for i, e in enumerate(json.load(open(os.path.join(here, 'corpus2-congelado.json')))):
    if e['form'] == 'matchexpr': items.append(('c2', i, e['wrapped']))
for sd in ('7021993', '4417061', '9908255'):
    for i, e in enumerate(json.load(open(os.path.join(E.REF, '..', '..', '..', '..', 'b-d4', 'corp5-%s.json' % sd)))):
        if e['form'] == 'matchexpr': items.append(('s' + sd, i, e['w']))
def arm2(w):
    a = w.index('None=>{value: ') + 14; b = w.index(';}}', a); return a, b
def work(it):
    w = it[2]; a, b = arm2(w)
    f = lambda sig: sorted((d[0], d[2], d[3]) for d in sig if d != 'ERR' and a <= d[2] < b)
    return f(E.run_bin(M, w, P3)), f(E.run_bin(C, w, P3, FL)), w
with ThreadPoolExecutor(6) as ex: res = list(ex.map(work, items))
tab = collections.Counter(); exs = []
for it, (m, c, w) in zip(items, res):
    tab[(it[0], 'n')] += 1
    if m != c:
        tab[(it[0], 'difiere_2o_brazo')] += 1; exs.append((it[0], it[1], m, c))
for k in sorted(tab): print(k, tab[k])
print('TOTAL difiere en el 2o brazo:', len(exs))
for e in exs[:8]: print(e)
json.dump(exs, open(os.path.join(here, 'armdiff-last.json'), 'w'))
