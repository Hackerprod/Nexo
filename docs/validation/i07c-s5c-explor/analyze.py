#!/usr/bin/env python3
"""Tabla (A)-(D) por candidato. Uso: analyze.py CORPUS ORACULO RES.json BASE CAND [nivel=full|core] [--ej N]
(A) regresiones: base==oraculo y cand!=oraculo ; (B) arreglos: base!=oraculo y cand==oraculo
(C) ambos fallan igual: base!=orc, cand!=orc, base==cand ; (D) ambos fallan distinto.
"""
import sys, json
corpus = json.load(open(sys.argv[1])); orc = json.load(open(sys.argv[2])); res = json.load(open(sys.argv[3]))
base, cand = sys.argv[4], sys.argv[5]
lvl = sys.argv[6] if len(sys.argv) > 6 and not sys.argv[6].startswith('--') else 'full'
nex = 5
if '--ej' in sys.argv: nex = int(sys.argv[sys.argv.index('--ej') + 1])
def norm(s):
    if isinstance(s, str) or s is None: return s
    if lvl == 'core': return [tuple(d[:4]) for d in s]
    return [tuple(d) if not isinstance(d, tuple) else d for d in s]
def eq(a, b):
    if a is None or b is None or isinstance(a, str) or isinstance(b, str): return a == b
    return json.dumps(norm(a)) == json.dumps(norm(b))
PL = 78
cnt = {'A': [], 'B': [], 'C': [], 'D': [], 'ok': 0, 'err': 0}
for i, s in enumerate(corpus):
    o, b, c = orc[i], res[base][i], res[cand][i]
    if o is None: continue
    if isinstance(b, str) or isinstance(c, str): cnt['err'] += 1
    bo, co = eq(b, o), eq(c, o)
    if bo and not co: cnt['A'].append(i)
    elif not bo and co: cnt['B'].append(i)
    elif bo and co: cnt['ok'] += 1
    elif eq(b, c): cnt['C'].append(i)
    else: cnt['D'].append(i)
print('nivel', lvl, base, '->', cand, 'n', len(corpus))
print('ambos aciertan', cnt['ok'], '| (A) regresiones', len(cnt['A']), '| (B) arreglos', len(cnt['B']), '| (C) fallan igual', len(cnt['C']), '| (D) fallan distinto', len(cnt['D']), '| ERR', cnt['err'])
def show(k):
    for i in cnt[k][:nex]:
        print(' ', k, i, corpus[i][PL:-1]); print('     orc ', orc[i]); print('     base', res[base][i]); print('     cand', res[cand][i])
show('A')
if '--D' in sys.argv: show('D')
json.dump({k: cnt[k] for k in 'ABCD'}, open('last-analysis-%s-%s-%s.json' % (base, cand, lvl), 'w'))
