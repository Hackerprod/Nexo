#!/usr/bin/env python3
"""Ronda 3: evaluacion unificada de un binario/variante sobre todos los corpus y tabla A-D frente a main y oraculo.
Uso:
  eval3.py run TAG EXE[|FLAGS] [--only c1,c2,b,perm,t] [--prof2 f0-0002-b-v2,f0-0003-b-v1]   -> .scratch/res3/TAG.json
  eval3.py table BASE_TAG CAND_TAG [--ej N]
Corpus: c1 (corpus-congelado, 24.000), c2 (corpus2, 16.000, W con traslado y D con oraculo analogo), b (corpus-b), perm, t (corpus-t).
Nivel core = (codigo, subcausa, primario); full = firma completa (para D: sin relacion).
"""
import sys, json, os, subprocess, threading, collections
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import oracle2 as O2
import analyze2d as AD
REF = os.path.join(here, '..', '..', '..', '.scratch', 'ref', 'docs', 'validation', 'i07c-s5c', 'b-d2')
RES = os.path.join(here, '..', '..', '..', '.scratch', 'res3')
os.makedirs(RES, exist_ok=True)
WORK = os.path.join(os.environ.get('TEMP', '.'), 'ev3'); os.makedirs(WORK, exist_ok=True)
P1 = 'f0-0002-b-v2'


def key(x):
    return tuple('' if v is None else v for v in x[:4]) + (str(x[4:]),)


def sig_diag(d):
    rel = None
    for r in d.get('related_spans', []):
        if r['role'] == 'expected_type':
            rel = (r['span']['start'], r['span']['end'])
    data = d.get('data') or {}
    return (d['code'], d.get('subcause'), d['primary_span']['start'], d['primary_span']['end'],
            data.get('expected_type') or None, data.get('found_type') or None, rel)


def run_bin(exe, src, prof, xv=None):
    f = os.path.join(WORK, 'f%d.n' % threading.get_ident())
    with open(f, 'w', newline='') as fh:
        fh.write(src)
    env = dict(os.environ, NEXO_XV=xv) if xv is not None else None
    p = subprocess.run([exe, 'check', '--format', 'json', '--profile', prof, f], capture_output=True, text=True, timeout=120, env=env)
    try:
        j = json.loads(p.stdout)
    except Exception:
        return ['ERR']
    return sorted((sig_diag(d) for d in j.get('diagnostics', [])), key=key)


def oracle_flat(src):
    try:
        r = O2.derive(src)
    except Exception:
        return None
    if r.get('unmodeled') or r.get('error'):
        return None
    out = []
    for d in r['diagnostics']:
        rel = tuple(d['rel_expected']) if d.get('rel_expected') else None
        out.append((d['code'], d.get('subcause'), d['primary'][0], d['primary'][1], d.get('expected_type') or None, d.get('found_type') or None, rel))
    return sorted(out, key=key)


def oracle_w(e):
    r = oracle_flat(e['flat'])
    if r is None:
        return None
    out = []

    def mp(pos):
        for fs, fe, ws in e['segs']:
            if fs <= pos < fe or (pos == fe and fs < fe):
                return pos - fs + ws
        return None
    for d in r:
        a, b = mp(d[2]), mp(d[3])
        rel = None
        if d[6]:
            ra, rb = mp(d[6][0]), mp(d[6][1])
            if ra is None or rb is None:
                return None
            rel = (ra, rb)
        if a is None or b is None:
            return None
        out.append((d[0], d[1], a, b, d[4], d[5], rel))
    return sorted(out, key=key)


def corpora(only):
    C = {}
    if 'c1' in only:
        C['c1'] = (json.load(open(os.path.join(here, 'corpus-congelado.json'))), json.load(open(os.path.join(here, 'oraculo-res.json'))), [P1])
    c2 = None
    if 'c2' in only:
        c2 = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
    if 'b' in only:
        s = json.load(open(os.path.join(REF, 'corpus-b.json'))); C['b'] = (s, None, [P1])
    if 'perm' in only:
        s = json.load(open(os.path.join(REF, 'perm.json')))['progs']; C['perm'] = (s, None, [P1])
    for sd in ('7021993', '4417061', '9908255'):
        if 's5' in only:
            d = json.load(open(os.path.join(REF, '..', '..', '..', '..', 'b-d4', 'corp5-%s.json' % sd)))
            C['s' + sd] = ([e['w'] for e in d], None, ['f0-0003-b-v1'])
    if 't' in only:
        s = json.load(open(os.path.join(REF, 'corpus-t.json'))); C['t'] = (s, None, [P1])
    return C, c2


def oracles(name, srcs):
    cache = os.path.join(RES, 'orc-%s.json' % name)
    if os.path.exists(cache):
        return [None if x is None else [tuple(t) for t in x] for x in json.load(open(cache))]
    with ThreadPoolExecutor(1) as ex:
        o = list(ex.map(oracle_flat, srcs))
    json.dump(o, open(cache, 'w'))
    return o


def cmd_run():
    tag, exe = sys.argv[2], sys.argv[3]
    only = (sys.argv[sys.argv.index('--only') + 1] if '--only' in sys.argv else 'c1,c2,b,perm,t,s5').split(',')
    prof2 = (sys.argv[sys.argv.index('--prof2') + 1] if '--prof2' in sys.argv else P1).split(',')
    xv = None
    if '|' in exe:
        exe, xv = exe.split('|', 1)
    C, c2 = corpora(only)
    out = {}
    for name, (srcs, orc, profs) in C.items():
        for prof in profs:
            with ThreadPoolExecutor(6) as ex:
                out['%s|%s' % (name, prof)] = list(ex.map(lambda s: run_bin(exe, s, prof, xv), srcs))
            print(tag, name, prof, flush=True)
    if c2 is not None:
        for prof in prof2:
            with ThreadPoolExecutor(6) as ex:
                out['c2|%s' % prof] = list(ex.map(lambda e: run_bin(exe, e['wrapped'], prof, xv), c2))
            print(tag, 'c2', prof, flush=True)
    json.dump(out, open(os.path.join(RES, tag + '.json'), 'w'))


def cmd_table():
    base, cand = json.load(open(os.path.join(RES, sys.argv[2] + '.json'))), json.load(open(os.path.join(RES, sys.argv[3] + '.json')))
    nex = int(sys.argv[sys.argv.index('--ej') + 1]) if '--ej' in sys.argv else 0
    rows = []
    regs = {}
    for k in cand:
        if k not in base:
            continue
        name, prof = k.split('|')
        if name == 'c1':
            srcs = json.load(open(os.path.join(here, 'corpus-congelado.json'))); O = [None if x is None else [tuple(t) for t in x] for x in json.load(open(os.path.join(here, 'oraculo-res.json')))]
            kinds = ['all'] * len(srcs)
        elif name == 'c2':
            c2 = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
            cache = os.path.join(RES, 'orc-c2.json')
            if os.path.exists(cache):
                O = [None if x is None else [tuple(t) for t in x] for x in json.load(open(cache))]
            else:
                O = [oracle_w(e) if e['group'] == 'W' else AD.analog(e) for e in c2]
                json.dump(O, open(cache, 'w'))
            srcs = [e['wrapped'] for e in c2]
            kinds = [e['form'] for e in c2]
        elif name.startswith('s'):
            d = json.load(open(os.path.join(REF, '..', '..', '..', '..', 'b-d4', 'corp5-%s.json' % name[1:])))
            srcs = [e['w'] for e in d]
            cache = os.path.join(RES, 'orc-%s.json' % name)
            if os.path.exists(cache):
                O = [None if x is None else [tuple(t) for t in x] for x in json.load(open(cache))]
            else:
                O = [oracle_flat(e['flat']) if e['form'] in ('flat', 'blockstyle') else None for e in d]
                json.dump(O, open(cache, 'w'))
            kinds = [e['form'] for e in d]
        else:
            fn = {'b': lambda: json.load(open(os.path.join(REF, 'corpus-b.json'))), 'perm': lambda: json.load(open(os.path.join(REF, 'perm.json')))['progs'], 't': lambda: json.load(open(os.path.join(REF, 'corpus-t.json')))}[name]
            srcs = fn(); O = oracles(name, srcs); kinds = ['all'] * len(srcs)
        groups = collections.defaultdict(list)
        for i in range(len(srcs)):
            groups[kinds[i] if (name == 'c2' or name.startswith('s')) else 'all'].append(i)
        for g, idx in sorted(groups.items()):
            for lvl in ('core', 'full'):
                A = Bn = Cn = Dn = ok = nm = 0
                aex = []
                for i in idx:
                    o = O[i]
                    if o is None:
                        nm += 1; continue
                    m, c = base[k][i], cand[k][i]
                    if lvl == 'core': fn_ = lambda s: json.dumps([x[:4] for x in s])
                    elif name == 'c2' and g in ('annot', 'arg', 'ret', 'option', 'result', 'takeempty', 'matchexpr'): fn_ = lambda s: json.dumps([x[:6] for x in s])
                    else: fn_ = lambda s: json.dumps(s)
                    if name == 'c2' and g in ('annot', 'arg', 'ret', 'option', 'result', 'takeempty', 'matchexpr'):
                        o = [x[:6] if len(x) >= 6 else x for x in o]
                    mo, co = fn_(m) == fn_(o), fn_(c) == fn_(o)
                    if mo and not co: A += 1; aex.append(i)
                    elif not mo and co: Bn += 1
                    elif mo and co: ok += 1
                    elif fn_(m) == fn_(c): Cn += 1
                    else: Dn += 1
                rows.append((name, prof, g, lvl, len(idx), nm, ok, A, Bn, Cn, Dn))
                if A and lvl == 'core':
                    regs[(name, prof, g)] = aex
    print('| corpus | perfil | forma | nivel | n | sin oraculo | ok | A | B | C | D |'); print('|---|---|---|---|---|---|---|---|---|---|---|')
    for r in rows:
        if r[2] == 'all' or r[7] or '--all' in sys.argv:
            print('| ' + ' | '.join(str(x) for x in r) + ' |')
    tot = sum(len(v) for v in regs.values())
    print('REGRESIONES core (A) total:', tot, {'/'.join(k): v[:6] for k, v in regs.items()})
    json.dump({'/'.join(k): v for k, v in regs.items()}, open(os.path.join(RES, 'regs-%s-%s.json' % (sys.argv[2], sys.argv[3])), 'w'))


if __name__ == '__main__':
    {'run': cmd_run, 'table': cmd_table}[sys.argv[1]]()
