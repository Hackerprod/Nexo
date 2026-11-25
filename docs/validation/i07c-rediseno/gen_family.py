#!/usr/bin/env python3
"""Generador de la FAMILIA COMPLETA (exploracion S5c; desechable). Determinista por semilla.
Uso: python -B gen_family.py SEMILLA N SALIDA.json   -> imprime SHA-256 del fichero escrito.
Programa = prefijo + [asignaciones antes] + contexto(union) + [asignaciones despues] + '}'.
Union = arbol binario de `if p then A else B` (parentesis siempre), profundidad de anidamiento 1..3, hojas
{q,r,s,1,2,true,ar,aar,r3,[q,r],r[0],[q]}; anidados en 1.a/2.a/ultima posicion (el arbol es aleatorio y se
reparte por rechazo); variable pendiente en then y else; asignaciones {ninguna, compatible, incompatible} antes
o despues, ambos ordenes; contexto {let, argumento, elemento de array, ==, anotacion}.
"""
import sys, json, random, hashlib

LEAVES = ['q', 'r', 's', '1', '2', 'true', 'ar', 'aar', 'r3', '[q,r]', 'r[0]', '[q]']
LEAF_W = [5, 5, 2, 3, 1, 2, 3, 2, 1, 2, 2, 1]
ASGS = ['q=[[ar]];', 'q=[ar];', 'q=[[3]];', 'q=[3];', 'q=ar;', 'r=[true];', 'r=[[ar]];', 'r=[ar];', 'r=[3];',
        'r=ar;', 's=[[ar]];', 's=[true];', 'q=r;', 'r=q;']
CTX = ['let a0=%s;', 'let a0=%s;', 'let a0=%s;', 'Print(%s);', 'let a0=[%s];', 'let a0=%s==ar;', 'let a0=%s==aar;',
       'let a0:I64=%s;']


def tree(rng, depth):
    # arbol con profundidad de anidamiento exacta <= depth
    if depth == 0:
        return rng.choices(LEAVES, LEAF_W)[0]
    a_nest = rng.random() < 0.5
    b_nest = rng.random() < 0.5
    return ('if', tree(rng, depth - 1) if a_nest else tree(rng, 0),
            tree(rng, depth - 1) if b_nest else tree(rng, 0))


def depth_of(t):
    return 0 if isinstance(t, str) else 1 + max(depth_of(t[1]), depth_of(t[2]))


def leaves_of(t):
    return [t] if isinstance(t, str) else leaves_of(t[1]) + leaves_of(t[2])


def render(t):
    if isinstance(t, str):
        return t
    return '(if p then %s else %s)' % (render(t[1]), render(t[2]))


def make(rng):
    d = rng.choice([1, 1, 2, 2, 2, 3, 3])
    t = tree(rng, d)
    if depth_of(t) < 1:
        t = ('if', t, rng.choice(LEAVES))
    lv = leaves_of(t)
    if len(lv) > 6:
        return None
    # fuerza variable pendiente presente en la mayoria (en then o else)
    used = set(lv)
    pre = 'fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];'
    if 's' in used or rng.random() < 0.1:
        pre += 'var s=[];'
    n_as = rng.choice([0, 0, 1, 1, 2, 2])
    asg = rng.sample(ASGS, n_as)
    if 's' not in pre:
        asg = [a for a in asg if not a.startswith('s=')]
    split = rng.randint(0, len(asg))  # las primeras `split` van antes (en el orden dado), el resto despues
    before, after = asg[:split], asg[split:]
    ctx = rng.choice(CTX)
    u = render(t)
    if ctx.startswith('let a0=%s==') or ctx.startswith('let a0=[') or ctx.startswith('Print'):
        pass
    return pre + ''.join(before) + (ctx % u) + ''.join(after) + '}'


def main():
    seed, n, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    rng = random.Random(seed)
    seen, progs = set(), []
    guard = 0
    while len(progs) < n and guard < n * 50:
        guard += 1
        s = make(rng)
        if s is None or s in seen:
            continue
        seen.add(s)
        progs.append(s)
    with open(out, 'w', newline='') as f:
        json.dump(progs, f, indent=0)
    h = hashlib.sha256(open(out, 'rb').read()).hexdigest()
    print(len(progs), 'programas', 'seed', seed, 'sha256', h)


main()
