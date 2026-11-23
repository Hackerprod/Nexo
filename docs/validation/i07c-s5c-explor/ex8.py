#!/usr/bin/env python3
"""Ronda 8: por pieza excluida, programas donde main falla (core != oraculo), el conjunto completo acierta y el conjunto sin la pieza falla; cifras y 2 ejemplos minimos."""
import sys, os, json
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
R = E.RES
full = json.load(open(os.path.join(R, 'full.json'))); main = json.load(open(os.path.join(R, 'main.json')))
core = lambda r: json.dumps([x[:4] for x in r])
def load_srcs_orc(k):
    name, prof = k.split('|')
    if name == 'c1':
        return json.load(open(os.path.join(here, 'corpus-congelado.json'))), [None if x is None else [tuple(t) for t in x] for x in json.load(open(os.path.join(here, 'oraculo-res.json')))]
    if name == 'c2':
        return [e['wrapped'] for e in json.load(open(os.path.join(here, 'corpus2-congelado.json')))], [None if x is None else [tuple(t) for t in x] for x in json.load(open(os.path.join(R, 'orc-c2.json')))]
    if name.startswith('s'):
        return [e['w'] for e in json.load(open(os.path.join(R, '..', 'ref', 'b-d4', 'corp5-%s.json' % name[1:])))], [None if x is None else [tuple(t) for t in x] for x in json.load(open(os.path.join(R, 'orc-%s.json' % name)))]
    fn = {'b': lambda: json.load(open(os.path.join(E.REF, 'corpus-b.json'))), 'perm': lambda: json.load(open(os.path.join(E.REF, 'perm.json')))['progs'], 't': lambda: json.load(open(os.path.join(E.REF, 'corpus-t.json')))}[name]
    return fn(), [None if x is None else [tuple(t) for t in x] for x in json.load(open(os.path.join(R, 'orc-%s.json' % name)))]
pieces = sys.argv[1:]
cache = {k: load_srcs_orc(k) for k in full}
out = {}
for p in pieces:
    X = json.load(open(os.path.join(R, 'no_%s.json' % p)))
    n = 0; nreg = 0; ex = []
    for k in full:
        S, O = cache[k]
        for i in range(len(S)):
            o = O[i]
            if o is None: continue
            oc = core([list(t) for t in o])
            m, f, x = core(main[k][i]), core(full[k][i]), core(X[k][i])
            if m != oc and f == oc and x != oc:
                n += 1; ex.append((len(S[i]), k, i))
            if m == oc and x != oc: nreg += 1
    ex.sort()
    out[p] = {'arreglos_perdidos': n, 'regresiones_sin_la_pieza': nreg, 'ejemplos': [(k, i) for _, k, i in ex[:2]]}
    print(p, out[p], flush=True)
    for _, k, i in ex[:2]:
        S, O = cache[k]
        print('  ##', k, i, '\n  ', S[i]); print('   main   ', [x[:4] for x in main[k][i]]); print('   oraculo', [x[:4] for x in cache[k][1][i]]); print('   sin_%s' % p, [x[:4] for x in X[k][i]]); print('   completo', [x[:4] for x in full[k][i]])
json.dump(out, open(os.path.join(here, 'piezas-excluidas-cifras.json'), 'w'), indent=1)
