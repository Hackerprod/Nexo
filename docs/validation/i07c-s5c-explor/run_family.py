#!/usr/bin/env python3
"""Ejecuta el corpus congelado contra el oraculo y N binarios; guarda firmas por programa (exploracion, desechable).
Uso: python -B run_family.py CORPUS.json OUT.json oraculo|bin NOMBRE=RUTA_EXE ...
Firma completa = (codigo, subcausa, inicio, fin, esperado, encontrado, rel_esperado) por diagnostico, ordenada.
"""
import sys, json, subprocess, os, threading
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '.scratch', 'ref', 'scripts'))
import i07c_s5c_oraculo as O

WORK = os.path.join(os.environ.get('TEMP', '.'), 'fam')
os.makedirs(WORK, exist_ok=True)
tl = threading.local()


def sig_bin(j):
    out = []
    for d in j['diagnostics']:
        rel = None
        for r in d.get('related_spans', []):
            if r['role'] == 'expected_type':
                rel = (r['span']['start'], r['span']['end'])
        data = d.get('data') or {}
        out.append((d['code'], d.get('subcause'), d['primary_span']['start'], d['primary_span']['end'],
                    data.get('expected_type') or None, data.get('found_type') or None, rel))
    return sorted(out, key=lambda x: tuple('' if v is None else v for v in x[:4]) + (str(x[4:]),))


def sig_orc(r):
    out = []
    for d in r['diagnostics']:
        rel = tuple(d['rel_expected']) if d.get('rel_expected') else None
        out.append((d['code'], d.get('subcause'), d['primary'][0], d['primary'][1],
                    d.get('expected_type') or None, d.get('found_type') or None, rel))
    return sorted(out, key=lambda x: tuple('' if v is None else v for v in x[:4]) + (str(x[4:]),))


def run_bin(exe, src, k):
    if not hasattr(tl, 'n'):
        tl.n = threading.get_ident()
    f = os.path.join(WORK, 'f%d.n' % tl.n)
    with open(f, 'w', newline='') as fh:
        fh.write(src)
    p = subprocess.run([exe, 'check', '--format', 'json', '--profile', 'f0-0002-b-v2', f], capture_output=True, text=True, timeout=60)
    try:
        j = json.loads(p.stdout)
    except Exception:
        return 'ERR:%s' % p.returncode
    s = sig_bin(j)
    # el archivo temporal aparece como file; los spans no dependen de el
    return s


def orc(src):
    try:
        r = O.derive(src)
    except Exception as e:
        return None
    if r.get('unmodeled') or r.get('error'):
        return None
    return sig_orc(r)


def main():
    corpus = json.load(open(sys.argv[1]))
    out, mode = sys.argv[2], sys.argv[3]
    if mode == 'oraculo':
        with ThreadPoolExecutor(1) as ex:
            res = list(ex.map(orc, corpus))
        json.dump(res, open(out, 'w'))
        print('oraculo', len(res), 'modelados', sum(1 for r in res if r is not None))
        return
    bins = [a.split('=', 1) for a in sys.argv[4:]]
    res = {}
    for name, exe in bins:
        with ThreadPoolExecutor(6) as ex:
            res[name] = list(ex.map(lambda s: run_bin(exe, s, 0), corpus))
        print(name, 'ok', flush=True)
    json.dump(res, open(out, 'w'))


main()
