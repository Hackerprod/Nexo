#!/usr/bin/env python3
"""Mide pasadas de planning por programa (exploracion). Uso: passes.py CORPUS EXE FLAGS"""
import sys, json, subprocess, os, threading, collections
from concurrent.futures import ThreadPoolExecutor
corpus = json.load(open(sys.argv[1])); exe = sys.argv[2]; flags = sys.argv[3]
WORK = os.path.join(os.environ.get('TEMP', '.'), 'pas'); os.makedirs(WORK, exist_ok=True)
tl = threading.local()
def run(s):
    f = os.path.join(WORK, 'p%d.n' % threading.get_ident())
    open(f, 'w', newline='').write(s)
    p = subprocess.run([exe, 'check', '--format', 'json', '--profile', 'f0-0002-b-v2', f], capture_output=True, text=True, env=dict(os.environ, NEXO_XV=flags + ',passes'))
    return [int(l.split()[1]) for l in p.stderr.splitlines() if l.startswith('PASSES')]
with ThreadPoolExecutor(4) as ex:
    res = list(ex.map(run, corpus))
h = collections.Counter(max(r) if r else 0 for r in res)
print('max pasadas observadas', max(h), 'histograma', sorted(h.items()))
