#!/usr/bin/env python3
"""Ronda 6: cambios de aceptacion entre main y el candidato en `check` y `run`, 3 perfiles, sobre la familia dirigida (var con
if/return/break) y las 3 semillas de B; y diferencial debug/release de `run`.
Uso: acept6.py MAIN_REL CAND_REL[|FLAGS] [MAIN_DBG CAND_DBG[|FLAGS]]   (rutas de exe; FLAGS = valor de NEXO_XV)
Aceptado = outcome distinto de rejected (y sin diagnosticos). Imprime recuentos y los primeros indices que cambian."""
import sys, os, json, subprocess, tempfile, threading
from concurrent.futures import ThreadPoolExecutor
here = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.abspath(os.path.join(here, '..', '..', '..'))
PROFS = ('f0-0002-b-v2', 'f0-0003-b-v1', 'f0-0005-b-v1')
SEEDS = ('7021993', '4417061', '9908255')


def spec(a):
    return tuple(a.split('|', 1)) if '|' in a else (a, None)


def run(exe, xv, cmd, src, prof):
    with tempfile.TemporaryDirectory() as td:
        f = os.path.join(td, 'p.n')
        open(f, 'w', newline='').write(src)
        env = dict(os.environ, NEXO_XV=xv) if xv is not None else None
        p = subprocess.run([exe, cmd, '--format', 'json', '--profile', prof, f], capture_output=True, text=True, timeout=120, env=env)
    try:
        j = json.loads(p.stdout)
    except Exception:
        return ('ERR', p.returncode)
    return ((j.get('outcome') or {}).get('kind'), p.returncode, len(j.get('diagnostics', [])))


def acc(r):
    return r[0] != 'rejected' and r[0] != 'ERR'


def sweep(name, progs, exes, cmds, profs):
    for prof in profs:
        for cmd in cmds:
            res = {}
            for tag, (exe, xv) in exes.items():
                with ThreadPoolExecutor(6) as ex:
                    res[tag] = list(ex.map(lambda s: run(exe, xv, cmd, s, prof), progs))
            a, b = res['main'], res['cand']
            ch = [i for i in range(len(progs)) if acc(a[i]) != acc(b[i])]
            errs = sum(1 for r in a + b if r[0] == 'ERR')
            print('%-14s %-13s %-5s n=%-5d main acepta %-5d cand acepta %-5d CAMBIOS DE ACEPTACION %d ERR %d %s' % (
                name, prof, cmd, len(progs), sum(acc(r) for r in a), sum(acc(r) for r in b), len(ch), errs, ch[:5]), flush=True)


def main():
    mr, cr = spec(sys.argv[1]), spec(sys.argv[2])
    exes = {'main': mr, 'cand': cr}
    dirigida = json.load(open(os.path.join(here, 'corpus-dirigido5.json')))
    dirigida = [x if isinstance(x, str) else x.get('w') or x.get('src') for x in dirigida]
    sweep('dirigida', dirigida, exes, ('check', 'run'), PROFS)
    for sd in SEEDS:
        d = json.load(open(os.path.join(ROOT, '.scratch', 'ref', 'b-d4', 'corp5-%s.json' % sd)))
        sweep('B-' + sd, [e['w'] for e in d], exes, ('check', 'run'), PROFS)
    if len(sys.argv) >= 5:
        md, cd = spec(sys.argv[3]), spec(sys.argv[4])
        # diferencial debug/release de `run` (resultado completo) para cada binario, perfil f0-0003 sobre la familia dirigida
        for tag, (rel, dbg) in (('main', (mr, md)), ('cand', (cr, cd))):
            with ThreadPoolExecutor(6) as ex:
                a = list(ex.map(lambda s: run(rel[0], rel[1], 'run', s, 'f0-0003-b-v1'), dirigida))
                b = list(ex.map(lambda s: run(dbg[0], dbg[1], 'run', s, 'f0-0003-b-v1'), dirigida))
            ch = [i for i in range(len(dirigida)) if a[i] != b[i]]
            print('debug/release run', tag, 'n=%d divergencias %d %s' % (len(dirigida), len(ch), ch[:5]), flush=True)


main()
