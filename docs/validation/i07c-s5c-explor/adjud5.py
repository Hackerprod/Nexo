#!/usr/bin/env python3
"""Adjudicacion uno a uno de una muestra de los 157 «propios del match» contra el oraculo analogo de la union del brazo 1.
Para cada programa: oraculo de `let a0=U;` (U = valor del primer brazo) con los mismos decl/pre/post, spans trasladados al
programa con `match`; se compara con main y candidato el conjunto de diagnosticos FUERA del segundo brazo.
Veredicto: mejora (cand == oraculo != main), regresion (main == oraculo != cand), neutra (misma salida fuera del 2.o brazo: la
diferencia esta en el 2.o brazo o no existe fuera), no adjudicable (ninguno coincide con el oraculo).
Uso: adjud5.py MAIN CAND N SEMILLA"""
import sys, os, json, random, collections
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
import analyze2d as AD
MAIN, CAND, N, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
P3 = 'f0-0003-b-v1'
ROOT = os.path.abspath(os.path.join(here, '..', '..', '..'))
c2 = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
prop = json.load(open(os.path.join(here, 'matchdiff-last.json')))
def src(corp, i):
    if corp == 'c2': return c2[i]['wrapped']
    return json.load(open(os.path.join(ROOT, '.scratch', 'ref', 'b-d4', 'corp5-%s.json' % corp[1:])))[i]['w']
pool = [(k.split('/')[0], e[0]) for k, v in prop.items() for e in v]
random.Random(SEED).shuffle(pool)
DECL = AD.DECL
def pieces(w):
    ds = w.index(DECL); de = ds + len(DECL)
    if w.startswith('var s=[];', de): de += 9
    k = w.index('let a0=match', de)
    us = w.index('value: ', k) + 7; ue = AD.balanced(w, us)
    ls = w.index('None=>{value: ', ue) + 14; le = w.index(';}}', ls)
    return ds, de, k, us, ue, ls, le
def keyd(d): return (d[0], d[2], d[3])
out = []
for corp, i in pool:
    if len(out) >= N: break
    w = src(corp, i)
    try: ds, de, k, us, ue, ls, le = pieces(w)
    except Exception: continue
    decl, pre, U, post = w[ds:de], w[de:k], w[us:ue], w[le + 4:-1]
    flat = 'fn Main(){' + decl + pre + 'let a0=' + U + ';' + post + '}'
    o = E.oracle_flat(flat)
    if o is None: continue
    # mapeo plano -> programa con match
    segs = [(len('fn Main(){'), len('fn Main(){') + len(decl), ds)]
    f0 = len('fn Main(){') + len(decl)
    segs.append((f0, f0 + len(pre), de)); f1 = f0 + len(pre) + len('let a0=')
    segs.append((f1, f1 + len(U), us)); f2 = f1 + len(U) + 1
    segs.append((f2, f2 + len(post), le + 4))
    def mp(pos):
        for fs, fe, wsx in segs:
            if fs <= pos < fe or (pos == fe and fs < fe): return pos - fs + wsx
    oo = set()
    ok = True
    for d in o:
        a, b = mp(d[2]), mp(d[3])
        if a is None or b is None: ok = False; break
        oo.add((d[0], a, b))
    if not ok: continue
    m, c = E.run_bin(MAIN, w, P3), E.run_bin(CAND, w, P3)
    fuera = lambda sig: {keyd(d) for d in sig if not (ls <= d[2] < le)}
    arm2 = lambda sig: {keyd(d) for d in sig if ls <= d[2] < le}
    fm, fc = fuera(m), fuera(c)
    if fm == fc: v = 'neutra' + (' (difiere solo el 2.o brazo)' if arm2(m) != arm2(c) else ' (sin diferencia fuera del 2.o brazo)')
    elif fc == oo and fm != oo: v = 'mejora'
    elif fm == oo and fc != oo: v = 'regresion'
    else: v = 'no adjudicable'
    out.append({'corpus': corp, 'idx': i, 'veredicto': v, 'U': U, 'main_fuera': sorted(fm), 'cand_fuera': sorted(fc), 'oraculo_U': sorted(oo), 'arm2_main': sorted(arm2(m)), 'arm2_cand': sorted(arm2(c))})
cnt = collections.Counter(o['veredicto'].split(' ')[0] + (' ' + o['veredicto'].split(' ', 1)[1] if o['veredicto'].startswith('no') else '') for o in out)
print('adjudicados', len(out), dict(cnt))
for o in out: print(o['corpus'], o['idx'], o['veredicto'], '|', o['U'][:70])
json.dump(out, open(os.path.join(here, 'adjudicacion-157-muestra.json'), 'w'), indent=1)
