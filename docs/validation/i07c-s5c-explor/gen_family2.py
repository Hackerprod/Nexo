#!/usr/bin/env python3
"""Generador 2 (exploracion S5c, amplia el barrido): uniones/variables pendientes de la familia 1 dentro de formas nuevas.
Uso: python -B gen_family2.py SEMILLA N SALIDA.json   -> imprime SHA-256.
Cada elemento: {id, form, group, flat, wrapped, segs:[[flat_start,flat_end,wrapped_start]]}.
group W (envoltorio con tipado equivalente): el oraculo deriva el programa plano `flat` y sus spans se
trasladan al programa `wrapped` por segmentos (decl, pre, ctx, post). group D (contexto nuevo): solo diferencial main/2f.
Formas W: fn, fnparam, while, if, match, fn+while, fn+match, blockstyle (hojas con bloque value), flat.
Formas D: annot, arg, ret, option, result, matchexpr, takeempty.
"""
import sys, json, random, hashlib

LEAVES = ['q', 'r', 's', '1', '2', 'true', 'ar', 'aar', 'r3', '[q,r]', 'r[0]', '[q]']
LEAF_W = [5, 5, 2, 3, 1, 2, 3, 2, 1, 2, 2, 1]
ASGS = ['q=[[ar]];', 'q=[ar];', 'q=[[3]];', 'q=[3];', 'q=ar;', 'r=[true];', 'r=[[ar]];', 'r=[ar];', 'r=[3];',
        'r=ar;', 's=[[ar]];', 's=[true];', 'q=r;', 'r=q;']
CTX = ['let a0=%s;', 'let a0=%s;', 'let a0=%s;', 'Print(%s);', 'let a0=[%s];', 'let a0=%s==ar;', 'let a0=%s==aar;']
DECL = 'let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];'


def tree(rng, depth):
    if depth == 0:
        return rng.choices(LEAVES, LEAF_W)[0]
    return ('if', tree(rng, depth - 1) if rng.random() < 0.5 else tree(rng, 0),
            tree(rng, depth - 1) if rng.random() < 0.5 else tree(rng, 0))


def depth_of(t):
    return 0 if isinstance(t, str) else 1 + max(depth_of(t[1]), depth_of(t[2]))


def leaves_of(t):
    return [t] if isinstance(t, str) else leaves_of(t[1]) + leaves_of(t[2])


def render(t, rng, block):
    if isinstance(t, str):
        return t
    a, b = render(t[1], rng, block), render(t[2], rng, block)
    if block:
        if rng.random() < 0.5:
            a = '{ value: %s; }' % a
        if rng.random() < 0.5:
            b = '{ value: %s; }' % b
    return '(if p then %s else %s)' % (a, b)


def build(rng, idx):
    d = rng.choice([1, 1, 2, 2, 2, 3])
    t = tree(rng, d)
    if depth_of(t) < 1:
        t = ('if', t, rng.choice(LEAVES))
    lv = leaves_of(t)
    if len(lv) > 6:
        return None
    decl = DECL + ('var s=[];' if ('s' in lv or rng.random() < 0.1) else '')
    asg = rng.sample(ASGS, rng.choice([0, 0, 1, 1, 2, 2]))
    if 'var s' not in decl:
        asg = [a for a in asg if not a.startswith('s=')]
    split = rng.randint(0, len(asg))
    pre, post = ''.join(asg[:split]), ''.join(asg[split:])
    if rng.random() < 0.6:
        form = rng.choice(['fn', 'fnparam', 'while', 'if', 'match', 'fn+while', 'fn+match', 'blockstyle', 'flat'])
        group = 'W'
    else:
        form = rng.choice(['annot', 'arg', 'ret', 'option', 'result', 'matchexpr', 'takeempty'])
        group = 'D'
    U = render(t, rng, form == 'blockstyle')
    ctxa, ctxb = rng.choice(CTX).split('%s')
    flat = 'fn Main(){' + decl + pre + ctxa + U + ctxb + post + '}'
    if group == 'W':
        head, tail, wo, wc = 'fn Main(){', '}', '', ''
        if form in ('fn', 'fn+while', 'fn+match'):
            head, tail = 'fn Helper(){', '}\nfn Main(){Helper();}'
        if form == 'fnparam':
            head, tail = 'fn Helper(z:view I64){', '}\nfn Main(){Helper(1);}'
        if form in ('while', 'fn+while'):
            wo, wc = 'while p{', 'break;}'
        if form == 'if':
            wo, wc = 'if p{', '}'
        if form in ('match', 'fn+match'):
            wo, wc = 'let o:Option<I64>=Some(1);match move o{Some(n)=>{', '},None=>{}}'
        wrapped = head + decl + wo + pre + ctxa + U + ctxb + post + wc + tail
        segs = []
        cur_f, cur_w = len('fn Main(){'), len(head)
        for name, txt in (('decl', decl), ('pre', pre), ('ctx', ctxa + U + ctxb), ('post', post)):
            if name == 'pre':
                cur_w += len(wo)
            segs.append([cur_f, cur_f + len(txt), cur_w])
            cur_f += len(txt)
            cur_w += len(txt)
        return dict(id=idx, form=form, group=group, flat=flat, wrapped=wrapped, segs=segs)
    leaf2 = rng.choice(LEAVES)
    n = rng.choice([1, 2, 3, 4])
    A = 'Array<' * n + 'I64' + '>' * n
    if form == 'annot':
        w = 'fn Main(){' + decl + pre + 'let a0:%s=%s;' % (A, U) + post + '}'
    elif form == 'arg':
        w = 'fn Take(a:view %s){}\nfn Main(){' % A + decl + pre + 'Take(%s);' % U + post + '}'
    elif form == 'ret':
        w = 'fn F(c:view Bool)->%s{' % A + decl + pre + post + 'return %s;}\nfn Main(){}' % U
    elif form == 'option':
        w = 'fn Main(){' + decl + pre + 'let o:Option<%s>=Some(%s);' % (A, U) + post + '}'
    elif form == 'result':
        w = 'fn R()->Result<%s,I64>{' % A + decl + pre + post + 'return Ok(%s);}\nfn Main(){}' % U
    elif form == 'matchexpr':
        w = ('fn Main(){let o:Option<I64>=Some(1);' + decl + pre +
             'let a0=match move o{Some(n)=>{value: %s;},None=>{value: %s;}};' % (U, leaf2) + post + '}')
    else:
        w = 'fn Take(a:view %s){}\nfn Main(){' % A + decl + pre + 'Take([]);Take(q);' + ctxa + U + ctxb + post + '}'
    return dict(id=idx, form=form, group=group, wrapped=w)


def main():
    seed, n, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    rng = random.Random(seed)
    seen, progs = set(), []
    while len(progs) < n:
        e = build(rng, len(progs))
        if e is None or e['wrapped'] in seen:
            continue
        seen.add(e['wrapped'])
        progs.append(e)
    with open(out, 'w', newline='') as f:
        json.dump(progs, f)
    print(len(progs), 'programas seed', seed, 'sha256', hashlib.sha256(open(out, 'rb').read()).hexdigest())


main()
