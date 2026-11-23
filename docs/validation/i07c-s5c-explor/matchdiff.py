#!/usr/bin/env python3
"""Diferencias PROPIAS DEL match entre main y un candidato.
Para cada programa con `match` se construye su analogo sin match (flat para `match`/`fn+match`; `(if p then U else HOJA)` para
`matchexpr`), se ejecuta main y candidato sobre el programa y sobre su analogo y se compara el delta de codigos
(candidato - main) de ambos: si difieren, la diferencia es propia del match.
Uso: matchdiff.py EXE_MAIN EXE_CAND FLAGS_CAND [--ej N]"""
import sys, os, re, json, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
import analyze2d as AD
M, C, FL = sys.argv[1], sys.argv[2], sys.argv[3]
nex = int(sys.argv[sys.argv.index('--ej') + 1]) if '--ej' in sys.argv else 3
P3 = 'f0-0003-b-v1'
items = []
c2 = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
for i, e in enumerate(c2):
    if e['form'] in ('match', 'fn+match'):
        items.append(('c2', i, e['form'], e['wrapped'], e['flat']))
    elif e['form'] == 'matchexpr':
        r = AD.d_flat(e)
        if r: items.append(('c2', i, 'matchexpr', e['wrapped'], r[0].replace('let a0:', 'let a0:') if False else None, r))
for sd in ('7021993', '4417061', '9908255'):
    d = json.load(open(os.path.join(E.REF, '..', '..', '..', '..', 'b-d4', 'corp5-%s.json' % sd)))
    for i, e in enumerate(d):
        if e['form'] == 'match':
            items.append(('s' + sd, i, 'match', e['w'], e['flat']))
        elif e['form'] == 'matchexpr':
            r = AD.d_flat({'form': 'matchexpr', 'wrapped': e['w']})
            if r: items.append(('s' + sd, i, 'matchexpr', e['w'], None, r))
def analog(it):
    return it[4] if it[4] else it[5][0]
def sig(exe, s, xv=None):
    return E.run_bin(exe, s, P3, xv)
def codes(x): return collections.Counter(d[0] for d in x if d != 'ERR')
def delta(a, b):  # b - a como pares (+/-)
    ca, cb = codes(a), codes(b)
    return tuple(sorted(((cb - ca) + collections.Counter()).elements())), tuple(sorted(((ca - cb) + collections.Counter()).elements()))
def work(it):
    w, a = it[3], analog(it)
    mw, cw = sig(M, w), sig(C, w, FL)
    ma, ca = sig(M, a), sig(C, a, FL)
    return mw, cw, ma, ca
with ThreadPoolExecutor(6) as ex:
    res = list(ex.map(work, items))
tab = collections.defaultdict(collections.Counter); ex_ = collections.defaultdict(list)
for it, (mw, cw, ma, ca) in zip(items, res):
    k = (it[0], it[2])
    tab[k]['n'] += 1
    if mw != cw:
        tab[k]['difiere_main'] += 1
        if delta(mw, cw) != delta(ma, ca):
            tab[k]['propia_del_match'] += 1
            ex_[k].append((it[1], delta(mw, cw), delta(ma, ca)))
        tab[k]['acepta_distinto'] += int((not mw) != (not cw))
print('| corpus | forma | n | main != 2g'+"'"+'+RM | propias del match | cambia aceptacion |'); print('|---|---|---|---|---|---|')
for k, c in sorted(tab.items()):
    print('| %s | %s | %d | %d | %d | %d |' % (k[0], k[1], c['n'], c['difiere_main'], c['propia_del_match'], c['acepta_distinto']))
tot = sum(c['propia_del_match'] for c in tab.values())
print('TOTAL propias del match:', tot)
for k, v in ex_.items():
    for e in v[:nex]: print(k, e)
json.dump({'%s/%s' % k: v for k, v in ex_.items()}, open(os.path.join(here, 'matchdiff-last.json'), 'w'))
