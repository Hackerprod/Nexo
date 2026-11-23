#!/usr/bin/env python3
"""Cambios de aceptacion (sin diagnosticos = aceptado) entre dos etiquetas de eval3 en todos los corpus. Uso: acept_corpora.py BASE CAND"""
import sys, os, json
here = os.path.dirname(os.path.abspath(__file__)); RES = os.path.join(here, '..', '..', '..', '.scratch', 'res3')
a, b = json.load(open(os.path.join(RES, sys.argv[1] + '.json'))), json.load(open(os.path.join(RES, sys.argv[2] + '.json')))
tot = 0
for k in b:
    if k not in a: continue
    ch = [i for i in range(len(b[k])) if (a[k][i] == []) != (b[k][i] == [])]
    acc = sum(1 for x in a[k] if x == [])
    print('%-22s n=%-6d main acepta %-5d cambios de aceptacion: %d %s' % (k, len(b[k]), acc, len(ch), ch[:5]))
    tot += len(ch)
print('TOTAL cambios de aceptacion:', tot)
