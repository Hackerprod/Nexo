#!/usr/bin/env python3
"""Tablas A-D por forma y perfil del barrido 2. Uso: analyze2.py CHECK.json [K2NOMBRE=k2_rel]"""
import sys, json, os, collections
here = os.path.dirname(os.path.abspath(__file__))
chk = json.load(open(sys.argv[1]))
K2 = sys.argv[2] if len(sys.argv) > 2 else 'k2_rel'
corpus = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
PROFS = sorted({k.split('|')[1] for k in chk if '|' in k and k.startswith('main_rel')})
core = lambda s: json.dumps([x[:4] for x in s])
full = lambda s: json.dumps(s)
BAD = {'NX-E0006', 'NX-E0002'}
out = []
reg = []
diffs = []
for P in PROFS:
    M, K = chk['main_rel|' + P], chk[K2 + '|' + P]
    forms = collections.defaultdict(lambda: collections.Counter())
    for i, e in enumerate(corpus):
        m, k = M[i], K[i]
        f = forms[e['form']]
        f['n'] += 1
        codes = {d[0] for d in m.get('sig', [])} | {d[0] for d in k.get('sig', [])}
        if codes & BAD:
            f['rechazado_perfil'] += 1
            if m.get('sig') != k.get('sig') or m['kind'] != k['kind']:
                f['rechazado_pero_difieren'] += 1
            continue
        if m['kind'] in ('ERR', 'TIMEOUT') or k['kind'] in ('ERR', 'TIMEOUT'):
            f['err_timeout'] += 1
            continue
        same = full(m['sig']) == full(k['sig']) and m['kind'] == k['kind']
        if not same:
            f['difieren_main_k2'] += 1
            diffs.append((P, i))
        if e['group'] == 'W':
            o = chk['oracle'][i]
            if o is None:
                f['sin_oraculo'] += 1
                continue
            for lvl, fn in (('core', core), ('full', full)):
                mo, ko = fn(m['sig']) == fn(o), fn(k['sig']) == fn(o)
                key = 'A' if mo and not ko else 'B' if (not mo and ko) else 'ok' if mo else ('C' if fn(m['sig']) == fn(k['sig']) else 'D')
                f[lvl + '_' + key] += 1
                if lvl == 'core' and key == 'A':
                    reg.append((P, i))
        else:
            f['D_igual'] += 0 if not same else 1
    out.append('### Perfil %s\n' % P)
    out.append('| forma | n | rechazado por perfil | comparados | difieren main/2f | core: ok / A / B / C / D | full: ok / A / B / C / D |')
    out.append('|---|---|---|---|---|---|---|')
    for name, f in sorted(forms.items()):
        comp = f['n'] - f['rechazado_perfil'] - f['err_timeout']
        c = ' / '.join(str(f['core_' + x]) for x in ('ok', 'A', 'B', 'C', 'D'))
        u = ' / '.join(str(f['full_' + x]) for x in ('ok', 'A', 'B', 'C', 'D'))
        out.append('| %s | %d | %d | %d | %d | %s | %s |' % (name, f['n'], f['rechazado_perfil'], comp, f['difieren_main_k2'], c, u))
    out.append('')
print('\n'.join(out))
json.dump({'regresiones_core': reg, 'difieren': diffs}, open(os.path.join(here, 'analisis2-%s.json' % K2), 'w'))
print('regresiones core (A):', len(reg), ' difieren main/2f:', len(diffs))
