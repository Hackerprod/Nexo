#!/usr/bin/env python3
"""Exporta los programas del corpus 1 que `main` Y 2f fallan en core frente al oraculo (los 322) y mide si
`e0103skip` (no emitir NX-E0103 cuyo constructor no tiene origen de literal `[]`) los arregla.
Uso: defectos322.py  (usa .scratch/bin/main-c.exe, xv15.exe)"""
import sys, json, os, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__))
src = open(os.path.join(here, 'run_family.py')).read().replace('\nmain()\n', '\n')
ns = {'__file__': os.path.join(here, 'run_family.py')}
exec(compile(src, 'rf', 'exec'), ns)
B = 'C:/projects/Nexo-i07c-d/.scratch/bin/'
corpus = json.load(open(os.path.join(here, 'corpus-congelado.json')))
orc = json.load(open(os.path.join(here, 'oraculo-res.json')))
core = lambda s: json.dumps([d[:4] for d in s])
def runall(exe, xv=None):
    with ThreadPoolExecutor(6) as ex:
        return list(ex.map(lambda s: ns['run_bin'](exe, s, 0, xv), corpus))
main = runall(B + 'main-c.exe')
k2 = runall(B + 'xv15.exe')
sk = runall(B + 'xv15.exe', 'pendfa,retfa,retract,anchw,nojoin,v6rej,atomic0,arrx,e0103skip')
out = []
cat = collections.Counter(); fixed = collections.Counter(); newbad = []
for i, s in enumerate(corpus):
    o = orc[i]
    if o is None: continue
    if core(main[i]) != core(o) and core(k2[i]) != core(o):
        oc = collections.Counter(d[0] for d in o); kc = collections.Counter(d[0] for d in k2[i])
        extra = kc - oc; miss = oc - kc
        key = ('extra ' + ','.join(sorted(extra.elements())) if extra else '') + (' | miss ' + ','.join(sorted(miss.elements())) if miss else '')
        if not extra and not miss: key = 'mismo codigo, distinto primario'
        cat[key] += 1
        fixed_by_skip = core(sk[i]) == core(o)
        fixed[key] += fixed_by_skip
        out.append({'idx': i, 'cuerpo': s[len('fn Main(){'):-1], 'categoria': key, 'oraculo': o, 'main': main[i], '2f': k2[i], '2f_e0103skip': sk[i], 'e0103skip_arregla': fixed_by_skip})
# e0103skip: regresiones nuevas (programas que 2f acierta y e0103skip no)
newbad = [i for i, s in enumerate(corpus) if orc[i] is not None and core(k2[i]) == core(orc[i]) and core(sk[i]) != core(orc[i])]
json.dump(out, open(os.path.join(here, 'defectos-previos-322.json'), 'w'), indent=1)
print('programas main y 2f fallan en core:', len(out))
for k, v in cat.most_common(): print(v, 'arregla e0103skip', fixed[k], '|', k)
print('e0103skip arregla en total', sum(fixed.values()), '; regresiones nuevas de e0103skip frente a 2f (2f acierta y skip no):', len(newbad), newbad[:10])
