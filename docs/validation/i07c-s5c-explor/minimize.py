#!/usr/bin/env python3
"""Programa minimo por pieza: reduce un programa de la ablacion (main == oraculo, sin la pieza != oraculo en core).
Uso: minimize.py EXE FLAGS_FINALES PIEZA CORPUS IDX   (CORPUS: c1|b|perm|t)"""
import sys, os, re, json
here = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, here)
import eval3 as E
exe, flags, piece, corp, idx = sys.argv[1], sys.argv[2].split(','), sys.argv[3], sys.argv[4], int(sys.argv[5])
REF = E.REF
srcs = {'c1': lambda: json.load(open(os.path.join(here, 'corpus-congelado.json'))), 'b': lambda: json.load(open(os.path.join(REF, 'corpus-b.json'))),
        'perm': lambda: json.load(open(os.path.join(REF, 'perm.json')))['progs'], 't': lambda: json.load(open(os.path.join(REF, 'corpus-t.json')))}[corp]()
MAIN = 'C:/projects/Nexo-i07c-d/.scratch/bin/main.exe'
core = lambda s: json.dumps([x[:4] for x in s])
abl = ','.join(f for f in flags if f != piece)
def bad(s):
    o = E.oracle_flat(s)
    if o is None: return False
    return core(E.run_bin(MAIN, s, E.P1)) == core(o) and core(E.run_bin(exe, s, E.P1, abl)) != core(o)
def split_if(t):
    body = t[len('(if p then '):-1]; d = 0
    for i, c in enumerate(body):
        if c in '([': d += 1
        elif c in ')]': d -= 1
        elif d == 0 and body.startswith(' else ', i): return body[:i], body[i + 6:]
def cands(s):
    out = []
    for m in re.finditer(r'(?<=;)[qrs]=[^;]*;', s): out.append(s[:m.start()] + s[m.end():])
    for m in re.finditer(r'\(if p then ', s):
        st = m.start(); d = 0
        for j in range(st, len(s)):
            if s[j] == '(': d += 1
            elif s[j] == ')':
                d -= 1
                if d == 0: break
        sp = split_if(s[st:j + 1])
        if sp:
            for x in sp: out.append(s[:st] + x + s[j + 1:])
    return out
s = srcs[idx]
if not bad(s): print('no reproduce'); sys.exit()
ch = True
while ch:
    ch = False
    for c in cands(s):
        if len(c) < len(s) and bad(c): s = c; ch = True; break
body = s[s.index('var r=[];') + 9:-1] if 'var r=[];' in s else s
o = E.oracle_flat(s)
print(piece, corp, idx); print('  ', body)
print('   oraculo=main:', [x[:6] for x in o]); print('   sin pieza  :', [x[:6] for x in E.run_bin(exe, s, E.P1, abl)])
