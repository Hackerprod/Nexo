#!/usr/bin/env python3
"""Barrido 2, grupo D con ORACULO ANALOGO (equivalencia por construccion, no derivada del ADR para la forma):
annot/arg/ret/option/result: el contexto externo determinado se modela como anotacion `let a0:Array^n<I64>=U;`;
takeempty: `Take(q)` con parametro Array^n<I64> se modela como `q=<literal de n capas>;` antes de la union.
Compara en nivel core (codigo, subcausa, primario) y typed (core + tipos esperado/encontrado), sin relacion.
Uso: analyze2d.py CHECK.json [k2_rel]"""
import sys, json, os, re, collections
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import oracle2 as O2
DECL = 'let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];'


def balanced(s, i):
    """indice (exclusivo) del cierre del grupo que abre en s[i]=='('"""
    d = 0
    for j in range(i, len(s)):
        if s[j] in '([': d += 1
        elif s[j] in ')]':
            d -= 1
            if d == 0: return j + 1
    raise ValueError


def d_flat(e):
    w, form = e['wrapped'], e['form']
    ds = w.index(DECL)
    de = ds + len(DECL)
    if w.startswith('var s=[];', de): de += 9
    decl = w[ds:de]
    pieces = [('fn Main(){', None), (decl, ds)]
    def lit(n): return '[' * n + '1' + ']' * n
    if form == 'annot':
        k = w.index('let a0:', de)
        eq = w.index('=(', k)
        A = w[k + 7:eq]
        us = eq + 1; ue = balanced(w, us)
        pre = w[de:k]; post = w[ue + 1:-1]
        pieces += [(pre, de), ('let a0:%s=' % A, None), (w[us:ue], us), (';', None), (post, ue + 1)]
    elif form == 'arg':
        A = re.match(r'fn Take\(a:view (.*)\)\{\}', w).group(1)
        k = w.index('Take(', de)
        us = k + 5; ue = balanced(w, us)
        pieces += [(w[de:k], de), ('let a0:%s=' % A, None), (w[us:ue], us), (';', None), (w[ue + 2:-1], ue + 2)]
    elif form == 'ret':
        A = re.match(r'fn F\(c:view Bool\)->(.*)\{let p', w).group(1)
        k = w.index('return ', de)
        us = k + 7; ue = balanced(w, us)
        pieces += [(w[de:k], de), ('let a0:%s=' % A, None), (w[us:ue], us), (';', None)]
    elif form == 'option':
        A = re.search(r'let o:Option<(.*)>=Some\(\(if', w).group(1)
        k = w.index('let o:Option<', de)
        us = w.index('Some(', k) + 5; ue = balanced(w, us)
        pieces += [(w[de:k], de), ('let a0:%s=' % A, None), (w[us:ue], us), (';', None), (w[ue + 2:-1], ue + 2)]
    elif form == 'result':
        A = re.match(r'fn R\(\)->Result<(.*),I64>\{let p', w).group(1)
        k = w.index('return Ok(', de)
        us = k + 10; ue = balanced(w, us)
        pieces += [(w[de:k], de), ('let a0:%s=' % A, None), (w[us:ue], us), (';', None)]
    elif form == 'takeempty':
        A = re.match(r'fn Take\(a:view (.*)\)\{\}', w).group(1)
        n = A.count('Array<')
        k = w.index('Take([]);Take(q);', de)
        c0 = k + len('Take([]);Take(q);')
        d = 0; c1 = None
        for j in range(c0, len(w)):
            if w[j] in '([': d += 1
            elif w[j] in ')]': d -= 1
            elif w[j] == ';' and d == 0: c1 = j + 1; break
        ctx = w[c0:c1]
        ui = ctx.index('(if p')
        ue = balanced(ctx, ui)
        pieces += [(w[de:k], de), ('q=%s;' % lit(n), None), (ctx[:ui], c0), (ctx[ui:ue], c0 + ui), (ctx[ue:], c0 + ue), (w[c1:-1], c1)]
    elif form == 'matchexpr':
        k = w.index('let a0=match', de)
        us = w.index('value: ', k) + 7; ue = balanced(w, us)
        ls = w.index('None=>{value: ', ue) + 14
        le = w.index(';}};', ls)
        pieces += [(w[de:k], de), ('let a0=(if p then ', None), (w[us:ue], us), (' else ', None), (w[ls:le], ls), (');', None), (w[le + 4:-1], le + 4)]
    else:
        return None
    pieces.append(('}', None))
    flat = ''.join(p for p, _ in pieces)
    segs = []; cur = 0
    for text, ws in pieces:
        if ws is not None and text:
            segs.append((cur, cur + len(text), ws))
        cur += len(text)
    return flat, segs


def key(x): return tuple('' if v is None else v for v in x[:4]) + (str(x[4:]),)


def analog(e):
    r = d_flat(e)
    if r is None: return None
    flat, segs = r
    try:
        o = O2.derive(flat)
    except Exception:
        return None
    if o.get('unmodeled') or o.get('error'): return None
    out = []
    def mp(pos):
        for fs, fe, ws in segs:
            if fs <= pos < fe or (pos == fe and fs < fe): return pos - fs + ws
        return None
    for d in o['diagnostics']:
        a, b = d['primary']
        ma, mb = mp(a), mp(b)
        if ma is None or mb is None: return None
        out.append((d['code'], d.get('subcause'), ma, mb, d.get('expected_type') or None, d.get('found_type') or None))
    return sorted(out, key=key)


if __name__ == '__main__':
    chk = json.load(open(sys.argv[1]))
    K2 = sys.argv[2] if len(sys.argv) > 2 else 'k2_rel'
    corpus = json.load(open(os.path.join(here, 'corpus2-congelado.json')))
    P = 'f0-0003-b-v1'
    core = lambda s: json.dumps([x[:4] for x in s]); typed = lambda s: json.dumps([x[:6] for x in s])
    tab = collections.defaultdict(collections.Counter); regs = []; ana = {}
    for i, e in enumerate(corpus):
        if e['group'] != 'D': continue
        o = analog(e)
        ana[i] = o
        f = tab[e['form']]; f['n'] += 1
        if o is None: f['sin_analogo'] += 1; continue
        m, k = chk['main_rel|' + P][i], chk[K2 + '|' + P][i]
        for lvl, fn in (('core', core), ('typed', typed)):
            mo, ko = fn(m['sig']) == fn(o), fn(k['sig']) == fn(o)
            key_ = 'A' if mo and not ko else 'B' if (not mo and ko) else 'ok' if mo else ('C' if fn(m['sig']) == fn(k['sig']) else 'D')
            f[lvl + '_' + key_] += 1
            if lvl == 'core' and key_ == 'A': regs.append(i)
    print('| forma | n | sin analogo | core: ok / A / B / C / D | typed: ok / A / B / C / D |'); print('|---|---|---|---|---|')
    for name, f in sorted(tab.items()):
        print('| %s | %d | %d | %s | %s |' % (name, f['n'], f['sin_analogo'], ' / '.join(str(f['core_' + x]) for x in ('ok', 'A', 'B', 'C', 'D')), ' / '.join(str(f['typed_' + x]) for x in ('ok', 'A', 'B', 'C', 'D'))))
    print('regresiones core (A):', len(regs), regs[:20])
    json.dump(regs, open(os.path.join(here, 'analisis2d-regs.json'), 'w'))
