#!/usr/bin/env python3
"""Familia dirigida a `var` con inicializador divergente / `return` / `break` / `while`, ciclos de arrays y asignaciones.
Uso: gen_dirigida5.py SEMILLA N SALIDA.json -> imprime SHA-256."""
import sys, json, random, hashlib
INIT = ['(if true then {return;} else {return;})', '(if true then {return;} else [1])', '(if p then [] else {return;})', '[]', '[1]', '[[]]', '1',
        '(if p then {return;} else {break;})', '[(if true then {return;} else {return;})]', '(if p then [] else [1])']
STM = ['v=[w];', 'w=[v];', 'v=a;', 'a=[v];', 'v=[v];', 'w=v;', 'v[0]=v;', 'AssertEq(v,w);', 'let z=(if p then v else w);', 'a=v;', 'let t=[v,w];', 'var u=[v];u=v;',
       'v=[a];', 'w=[a];', 'let y=v[0];', 'v=[1];', 'w=[true];']
def build(rng):
    iv, iw, ia = rng.choice(INIT), rng.choice(INIT), rng.choice(INIT)
    st = ''.join(rng.sample(STM, rng.choice([1, 2, 2, 3, 3, 4])))
    pre = 'let p=true;var v=%s;var w=%s;var a=%s;' % (iv, iw, ia)
    form = rng.choice(['flat', 'while', 'if', 'ret'])
    if form == 'flat': body = pre + st
    elif form == 'while': body = pre + 'while p{' + st + 'break;}'
    elif form == 'if': body = pre + 'if p{' + st + '}'
    else: body = pre + st + 'return;'
    return 'fn Main(){' + body + '}'
seed, n, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
rng = random.Random(seed); seen = set(); ps = []
while len(ps) < n:
    s = build(rng)
    if s in seen: continue
    seen.add(s); ps.append(s)
json.dump(ps, open(out, 'w'))
print(len(ps), 'programas semilla', seed, 'sha256', hashlib.sha256(open(out, 'rb').read()).hexdigest())
