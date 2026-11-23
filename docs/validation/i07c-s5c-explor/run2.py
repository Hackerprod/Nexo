#!/usr/bin/env python3
"""Barrido 2 (exploracion): corpus2 x perfiles x {check, run} x {main, 2f} x {debug, release}.
Uso: python -B run2.py FASE SALIDA.json [--prof P1,P2] [--limit N]
FASE: check  -> firmas de check (release) de main y 2f en los 3 perfiles (+ oraculo para el grupo W)
      run    -> run main vs 2f en debug y release sobre lo aceptado por check (lee la salida de la fase check)
"""
import sys, json, os, subprocess, threading
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__))
ref = os.path.join(here, '..', '..', '..', '.scratch', 'ref')
sys.path.insert(0, os.path.join(ref, 'scripts'))
import i07c_s5c_oraculo as O
BIN = 'C:/projects/Nexo-i07c-d/.scratch/bin/'
EXES = {'main_rel': BIN + 'main-c.exe', 'k2_rel': BIN + 'xv14.exe', 'main_dbg': BIN + 'main-dbg.exe', 'k2_dbg': BIN + 'k2-dbg.exe'}
PROFS = ['f0-0002-b-v2', 'f0-0003-b-v1', 'f0-0005-b-v1']
WORK = os.path.join(os.environ.get('TEMP', '.'), 'fam2'); os.makedirs(WORK, exist_ok=True)


def sig_diag(d):
    rel = None
    for r in d.get('related_spans', []):
        if r['role'] == 'expected_type':
            rel = (r['span']['start'], r['span']['end'])
    data = d.get('data') or {}
    return (d['code'], d.get('subcause'), d['primary_span']['start'], d['primary_span']['end'],
            data.get('expected_type') or None, data.get('found_type') or None, rel)


def key(x):
    return tuple('' if v is None else v for v in x[:4]) + (str(x[4:]),)


def run_one(exe, src, prof, cmd):
    f = os.path.join(WORK, 'f%d.n' % threading.get_ident())
    with open(f, 'w', newline='') as fh:
        fh.write(src)
    try:
        p = subprocess.run([exe, cmd, '--format', 'json', '--profile', prof, f], capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return {'kind': 'TIMEOUT'}
    try:
        j = json.loads(p.stdout)
    except Exception:
        return {'kind': 'ERR', 'rc': p.returncode, 'err': p.stderr[:200]}
    j['diagnostics'] = [dict(d, primary_span=dict(d['primary_span'], file='')) for d in j.get('diagnostics', [])]
    return {'kind': j.get('outcome', {}).get('kind'), 'rc': p.returncode,
            'sig': sorted((sig_diag(d) for d in j.get('diagnostics', [])), key=key),
            'full': json.dumps(j, sort_keys=True).replace(f.replace('\\', '/'), 'F').replace(f, 'F')}


def oracle_mapped(e):
    try:
        r = O.derive(e['flat'])
    except Exception:
        return None
    if r.get('unmodeled') or r.get('error'):
        return None
    out = []

    def mp(pos):
        for fs, fe, ws in e['segs']:
            if fs <= pos < fe or (pos == fe and fs < fe):
                return pos - fs + ws
        return None
    for d in r['diagnostics']:
        s, t = d['primary']
        ms, mt = mp(s), mp(t)
        rel = None
        if d.get('rel_expected'):
            a, b = d['rel_expected']
            ra, rb = mp(a), mp(b)
            if ra is None or rb is None:
                return None
            rel = (ra, rb)
        if ms is None or mt is None:
            return None
        out.append((d['code'], d.get('subcause'), ms, mt, d.get('expected_type') or None, d.get('found_type') or None, rel))
    return sorted(out, key=key)


def main():
    phase, outp = sys.argv[1], sys.argv[2]
    profs = PROFS
    if '--prof' in sys.argv:
        profs = sys.argv[sys.argv.index('--prof') + 1].split(',')
    corpus = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
    if '--limit' in sys.argv:
        corpus = corpus[:int(sys.argv[sys.argv.index('--limit') + 1])]
    res = {}
    if phase == 'check':
        with ThreadPoolExecutor(1) as ex:
            res['oracle'] = list(ex.map(lambda e: oracle_mapped(e) if e['group'] == 'W' else None, corpus))
        print('oraculo', sum(1 for x in res['oracle'] if x is not None), flush=True)
        for prof in profs:
            for name in ('main_rel', 'k2_rel'):
                with ThreadPoolExecutor(6) as ex:
                    res['%s|%s' % (name, prof)] = list(ex.map(lambda e: run_one(EXES[name], e['wrapped'], prof, 'check'), corpus))
                print(prof, name, flush=True)
    else:
        chk = json.load(open(sys.argv[3]))
        for prof in profs:
            idx = [i for i, r in enumerate(chk['main_rel|' + prof]) if r['kind'] == 'checked']
            # muestra de rechazados para run (mismos diagnosticos que check)
            rej = [i for i, r in enumerate(chk['main_rel|' + prof]) if r['kind'] != 'checked'][:600]
            sel = idx + rej
            res['sel|' + prof] = sel
            for name in ('main_rel', 'k2_rel', 'main_dbg', 'k2_dbg'):
                with ThreadPoolExecutor(6) as ex:
                    res['%s|%s' % (name, prof)] = list(ex.map(lambda i: run_one(EXES[name], corpus[i]['wrapped'], prof, 'run'), sel))
                print(prof, name, len(sel), flush=True)
    json.dump(res, open(outp, 'w'))


main()
