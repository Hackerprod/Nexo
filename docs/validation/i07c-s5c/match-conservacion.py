#!/usr/bin/env python3
"""Tabla de CONSERVACION de `match` como expresion con uniones (I07c-S5c). NO NORMATIVA.

Ejecuta el binario `main` (por defecto `.scratch/cand-bin/base-release.exe` del arbol Nexo-i07c) sobre cada programa de
`PROGRAMAS` y escribe `match-conservacion.json` con la salida ACTUAL, byte a byte: codigo, subcausa, primario, tipos y
relaciones de CADA diagnostico, mas el SHA-256 del binario. Escribe tambien `match-conservacion.sha256` (SHA-256 del JSON).

Uso:  python -B docs/validation/i07c-s5c/match-conservacion.py [RUTA_EXE]        regenera la tabla
      python -B docs/validation/i07c-s5c/match-conservacion.py --check [RUTA_EXE]  compara con la tabla versionada (exit 1 si difiere)

Marca de la tabla: «conservacion, no normativo»: el ADR no fija la union de los brazos de un `match` con valor cuando un brazo
contiene una union o una expresion con discrepancia o defecto interno. La tabla fija lo que hace la implementacion vigente
para detectar cambios; cualquier cambio exige Flujo A (no se edita la tabla para que pase una pieza).
"""
import hashlib, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..'))
DEFAULT_EXE = os.path.join(ROOT, '.scratch', 'cand-bin', 'base-release.exe')
OUT = os.path.join(HERE, 'match-conservacion.json')
SHA = os.path.join(HERE, 'match-conservacion.sha256')
NOTA = 'conservacion, no normativo'

S_QS = 'fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];var s=[];'
S_Q = 'fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];'
S_QR = 'fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];var r=[];'
S_3 = S_QS + 'let n0=1;'
PF = 'f0-0003-b-v1'


def M(pre, a1, a2='true'):
    return pre + 'let a0=match move o{Some(n)=>{value: %s;},None=>{value: %s;}};}' % (a1, a2)


def M3(a, b, c):
    return S_3 + 'let a0=match n0{1=>{value: %s;},2=>{value: %s;},_=>{value: %s;}};}' % (a, b, c)


PROGRAMAS = []


def add(pid, grupo, src, perfil=PF):
    PROGRAMAS.append((pid, grupo, perfil, src))


RE_1 = {
    'union-propia-denunciada': '(if p then ar else 1)',
    'union-propia-pendiente-else': '(if p then 1 else q)',
    'union-propia-pendiente-then': '(if p then q else 1)',
    'anidada-primera-pendiente': '(if p then (if p then 1 else q) else 2)',
    'anidada-primera-denunciada-externa-incompatible': '(if p then (if p then ar else 1) else 2)',
    'anidada-else-denunciada': '(if p then 1 else (if p then s else true))',
    'anidada-else-pendiente': '(if p then 1 else (if p then q else 2))',
    'anidada-else-denunciada2': '(if p then ar else (if p then 1 else q))',
    'union-valida': '(if p then 1 else 2)',
    'variable-pendiente': 'q',
    'literal-erroneo': '[1,true]',
    'union-q-s': '(if p then q else s)',
    'union-valida-ar-q': '(if p then ar else q)',
    'anidada-primera-q-1-ar': '(if p then (if p then q else 1) else ar)',
}
for k, a in RE_1.items():
    add('rejilla1-' + k, 'rejilla1', M(S_QS, a))

RE_2 = {
    'anidada-primera-denunciada-externa-compatible': ('(if p then (if p then ar else 1) else ar)', 'true'),
    'anidada-primera-denunciada-externa-compatible-brazo2-compatible': ('(if p then (if p then ar else 1) else ar)', 'ar'),
    'anidada-primera-pendiente-brazo2-compatible': ('(if p then (if p then 1 else q) else 2)', '3'),
    'anidada-primera-pendiente-brazo2-array': ('(if p then (if p then 1 else q) else 2)', 'ar'),
    'tres-niveles': ('(if p then (if p then (if p then 1 else q) else 2) else 3)', 'true'),
    'anidada-en-else-con-anidada': ('(if p then 2 else (if p then (if p then 1 else q) else 3))', 'true'),
    'match-anidado-con-anidada-pendiente': ('match move o{Some(m)=>{value: (if p then (if p then 1 else q) else 2);},None=>{value: 3;}}', 'true'),
    'match-anidado-con-discrepancia': ('match move o{Some(m)=>{value: (if p then ar else 1);},None=>{value: 3;}}', 'true'),
    'match-anidado-valido': ('match move o{Some(m)=>{value: 1;},None=>{value: 3;}}', 'true'),
    'union-propia-denunciada-brazo2-compatible': ('(if p then ar else 1)', 'ar'),
    'anidada-primera-pendiente-s-q': ('(if p then (if p then 1 else s) else q)', 'true'),
    'anidada-primera-pendiente-s-q-brazo2-compatible': ('(if p then (if p then 1 else s) else q)', '2'),
}
for k, (a, b) in RE_2.items():
    add('rejilla2-' + k, 'rejilla2', M(S_QS, a, b))

RE_3 = {
    'validos': ('1', '2', 'true'),
    'primero-anidado-pendiente': ('(if p then (if p then 1 else q) else 2)', '3', 'true'),
    'primero-propia-denunciada': ('(if p then ar else 1)', '2', 'true'),
    'segundo-anidado-pendiente': ('1', '(if p then (if p then 1 else q) else 2)', 'true'),
    'segundo-propia-denunciada': ('1', '(if p then ar else 1)', 'true'),
    'segundo-incompatible': ('1', 'true', 'ar'),
}
for k, t in RE_3.items():
    add('tres-brazos-' + k, 'tres-brazos', M3(*t))

# Los cuatro contraejemplos de B (I07c-lagunas-tres-textos-B-r2.md) y el de r1
add('B-178-anidada-pendiente-externa-q', 'contraejemplos-B', M(S_QS, '(if p then (if p then 1 else s) else q)', 'true'))
add('B-196-match-anidado-brazo-true-mas-1', 'contraejemplos-B',
    M(S_QR, 'match move o{Some(m)=>{value: 1;},None=>{value: true+1;}}', 'true'))
add('B-164-union-continuacion-true-mas-1', 'contraejemplos-B', M(S_QR, '(if p then true+1 else 1)', 'true'))
add('B-106-tres-brazos-enteros', 'contraejemplos-B',
    'fn Main(){let p=true;let b=match 1{1=>{value: 1;},2=>{value: (if p then 2 else true);},_=>{value: [1];}};}')
add('B-139-propia-pendiente', 'contraejemplos-B',
    M('fn Main(){let o:Option<I64>=Some(1);let p=true;var q=[];', '(if p then 1 else q)', 'true'))
add('B-161-anidada-else-pendiente-denunciada', 'contraejemplos-B',
    'fn Main(){let o:Option<I64>=Some(1);let p=true;var s=[];let a0=match move o{Some(n)=>{value: (if p then 1 else (if p then s else true));},None=>{value: true;}};}')

add('A-valido-127', 'laguna-A',
    'fn Main(){let o:Option<I64>=Some(1);let p=true;let a0=match move o{Some(n)=>{value: (if p then 1 else 2);},None=>{value: 3;}};}')
RE_A = {
    'anidada-pendiente-externa-compatible': ('(if p then (if p then 1 else q) else 2)', 'true'),
    'anidada-pendiente-brazo2-compatible': ('(if p then (if p then 1 else q) else 2)', '3'),
    'propia-pendiente': ('(if p then 1 else q)', 'true'),
    'propia-denunciada': ('(if p then ar else 1)', 'true'),
    'anidada-denunciada-externa-compatible': ('(if p then (if p then ar else 1) else ar)', 'true'),
    'anidada-denunciada-externa-incompatible': ('(if p then (if p then ar else 1) else 2)', 'true'),
    'match-anidado-valido': ('match move o{Some(m)=>{value: 1;},None=>{value: 3;}}', 'true'),
    'match-anidado-con-anidada-pendiente': ('match move o{Some(m)=>{value: (if p then (if p then 1 else q) else 2);},None=>{value: 3;}}', 'true'),
}
for k, (a, b) in RE_A.items():
    add('A-' + k, 'laguna-A', M(S_Q, a, b))

# Los 7 casos de X (R-C) se cargan de `match-conservacion-x.json` (datos: [id, fuente])
XF = os.path.join(HERE, 'match-conservacion-x.json')
if True:
    for pid, src in json.load(open(XF, encoding='utf-8')):
        add(pid, 'X-ronda3', src)


def diag(j):
    out = []
    for d in j['diagnostics']:
        dt = d.get('data') or {}
        out.append({
            'code': d['code'], 'subcause': d.get('subcause'),
            'primary': [d['primary_span']['start'], d['primary_span']['end']],
            'expected_type': dt.get('expected_type'), 'found_type': dt.get('found_type'),
            'relations': [{'role': r['role'], 'span': [r['span']['start'], r['span']['end']]} for r in d.get('related_spans', [])],
        })
    return out


def run(exe, src, perfil):
    with tempfile.TemporaryDirectory() as td:
        f = os.path.join(td, 'p.n')
        with open(f, 'w', newline='') as fh:
            fh.write(src)
        p = subprocess.run([exe, 'check', '--format', 'json', '--profile', perfil, f], capture_output=True, text=True, timeout=60)
    j = json.loads(p.stdout)
    return diag(j), (j.get('outcome') or {}).get('kind')


def build(exe):
    seen = set()
    filas = []
    for pid, grupo, perfil, src in PROGRAMAS:
        assert pid not in seen, pid
        seen.add(pid)
        d, outcome = run(exe, src, perfil)
        filas.append({'id': pid, 'grupo': grupo, 'perfil': perfil, 'bytes': len(src.encode('ascii')), 'fuente': src,
                      'outcome': outcome, 'diagnosticos': d, 'marca': NOTA})
    exe_sha = hashlib.sha256(open(exe, 'rb').read()).hexdigest()
    return {'marca': NOTA, 'binario_sha256': exe_sha, 'programas': len(filas), 'filas': filas}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    exe = os.path.abspath(args[0]) if args else DEFAULT_EXE
    tabla = build(exe)
    texto = json.dumps(tabla, indent=1, ensure_ascii=True, sort_keys=True) + '\n'
    if '--check' in sys.argv:
        viejo = json.load(open(OUT, encoding='utf-8'))
        errores = []
        n_prog, n_tabla, n_nueva = len(PROGRAMAS), len(viejo['filas']), len(tabla['filas'])
        if not (n_prog == n_tabla == n_nueva == viejo.get('programas')):
            errores.append('numero de filas: programas=%d tabla=%d ejecutadas=%d campo=%s' % (n_prog, n_tabla, n_nueva, viejo.get('programas')))
        ids_v, ids_n = [f['id'] for f in viejo['filas']], [f['id'] for f in tabla['filas']]
        if ids_v != ids_n:
            errores.append('ids distintos: solo en tabla %s; solo en programas %s' % (sorted(set(ids_v) - set(ids_n)), sorted(set(ids_n) - set(ids_v))))
        nuevo = {f['id']: f for f in tabla['filas']}
        for a_ in viejo['filas']:
            b_ = nuevo.get(a_['id'])
            if b_ is None:
                continue
            for campo in ('fuente', 'perfil', 'bytes', 'outcome', 'diagnosticos'):
                if a_.get(campo) != b_.get(campo):
                    errores.append('%s: difiere %s' % (a_['id'], campo))
        if viejo.get('binario_sha256') != tabla['binario_sha256']:
            print('aviso: SHA-256 del binario distinto del registrado (%s)' % tabla['binario_sha256'])
        print('programas', n_prog, 'filas', n_tabla, 'diferencias:', errores or 'ninguna')
        sys.exit(1 if errores else 0)
    with open(OUT, 'w', encoding='utf-8', newline='\n') as f:
        f.write(texto)
    with open(SHA, 'w', encoding='ascii', newline='\n') as f:
        f.write(hashlib.sha256(texto.encode('utf-8')).hexdigest() + '  match-conservacion.json\n')
    print('programas', tabla['programas'], 'binario', tabla['binario_sha256'])


main()
