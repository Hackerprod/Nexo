#!/usr/bin/env python3
"""EXTENSION (exploracion S5c ronda 5) de la tabla de CONSERVACION de `match` como expresion con uniones (I07c-S5c). NO NORMATIVA.

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
OUT = os.path.join(HERE, 'match-conservacion-ext.json')
SHA = os.path.join(HERE, 'match-conservacion-ext.sha256')
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
XF = os.path.join(HERE, 'match-conservacion-x.json')  # copia del fichero de la rama i07c-estructural-r2
if True:
    for pid, src in json.load(open(XF, encoding='utf-8')):
        add(pid, 'X-ronda3', src)



DF = os.path.join(HERE, 'match-conservacion-ext-decl.json')
DECL = json.load(open(DF, encoding='utf-8'))          # 28 del segundo brazo
DF21 = os.path.join(HERE, 'match-conservacion-ext-decl21.json')
DECL21 = json.load(open(DF21, encoding='utf-8'))      # 21 anadidos de RM-M3 fuera del segundo brazo
DFNA = os.path.join(HERE, 'match-conservacion-ext-noadj.json')
NOADJ = json.load(open(DFNA, encoding='utf-8'))       # 8 no adjudicables: listados aparte con ambas salidas, fuera de las 103
NOADJ_OUT = os.path.join(HERE, 'match-conservacion-ext-noadj-salidas.json')
GRUPO21 = 'diferencias_declaradas_rmm3_fuera_2o_brazo'
DECL_54 = {  # filas de las 54 originales cuyo valor del candidato difiere del de main (adjudicadas contra el oraculo)
    'rejilla1-anidada-else-denunciada': ('mejora_con_cita', 'ADR F0-0002 4.1.2.4 l.353 (una sola NX-E1303 por union, sobre la primera continuacion incompatible; las demas sin NX-E1303 propia) y l.367 (un conflicto pendiente de serializacion cuenta como ya denunciado): `s` pendiente incompatible con `1` ya denuncia; `true` no recibe E1303'),
    'B-161-anidada-else-pendiente-denunciada': ('mejora_con_cita', 'idem: l.353 y l.367'),
    'rejilla1-anidada-else-denunciada2': ('mejora_con_cita', 'ADR F0-0002 4.1.2.2 y l.301: NX-E0103 solo sobre un constructor fuente con elemento variable; `q` queda fijado por la union (referencia `ar`, hoja `q` compatible) y no emite el suyo'),
}
NOTA_D = 'diferencia declarada, no normativo'
PROFS_X = ('f0-0003-b-v1', 'f0-0005-b-v1')


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
    return {'outcome': (j.get('outcome') or {}).get('kind'), 'diagnosticos': diag(j)}


def todos():
    """(id, grupo, perfil, fuente) de las 103 filas: 51 de conservacion + 3 declaradas de las 54 + 28 declaradas + 21 declaradas de RM-M3."""
    filas = []
    for pid, grupo, perfil, src in PROGRAMAS:
        filas.append((pid, 'diferencias_declaradas_de_las_54' if pid in DECL_54 else 'conservacion', perfil, src))
    for d in DECL:
        filas.append((d['id'], 'diferencias_declaradas', d['perfil'], d['fuente']))
    for d in DECL21:
        filas.append((d['id'], GRUPO21, d['perfil'], d['fuente']))
    return filas


def regenerar(exe_main, exe_cand):
    seen, filas = set(), []
    decl = {d['id']: d for d in DECL + DECL21}
    for pid, grupo, perfil, src in todos():
        assert pid not in seen, pid
        seen.add(pid)
        m, c = run(exe_main, src, perfil), run(exe_cand, src, perfil)
        fila = {'id': pid, 'grupo': grupo, 'perfil': perfil, 'bytes': len(src.encode('ascii')), 'fuente': src}
        if grupo == 'conservacion':
            assert m == c, 'conservacion rota en %s' % pid
            fila.update(outcome=m['outcome'], diagnosticos=m['diagnosticos'], marca=NOTA)
        else:
            if grupo in ('diferencias_declaradas', GRUPO21):
                clase, cita = decl[pid]['clase'], decl[pid]['cita']
            else:
                clase, cita = DECL_54[pid]
            assert m != c, 'fila declarada sin diferencia: %s' % pid
            otro_m = [run(exe_main, src, p2) for p2 in PROFS_X]
            otro_c = [run(exe_cand, src, p2) for p2 in PROFS_X]
            fila.update(outcome=c['outcome'], diagnosticos=c['diagnosticos'], marca=NOTA_D, clase=clase, cita=cita,
                        main=m, candidato=c,
                        mismo_resultado_en_f0003_y_f0005=(otro_m[0] == otro_m[1] and otro_c[0] == otro_c[1]))
        filas.append(fila)
    na = []
    for d in NOADJ:
        m, c = run(exe_main, d['fuente'], d['perfil']), run(exe_cand, d['fuente'], d['perfil'])
        na.append({'id': d['id'], 'perfil': d['perfil'], 'fuente': d['fuente'], 'motivo': d['motivo'], 'main': m, 'candidato': c, 'difieren': m != c})
    texto_na = json.dumps({'marca': 'no adjudicables: ni mejora ni regresion; fuera de las 103 filas', 'filas': na}, indent=1, ensure_ascii=True, sort_keys=True) + '\n'
    open(NOADJ_OUT, 'w', encoding='utf-8', newline='\n').write(texto_na)
    return {'marca': NOTA + ' / ' + NOTA_D, 'binario_main_sha256': hashlib.sha256(open(exe_main, 'rb').read()).hexdigest(),
            'binario_candidato_sha256': hashlib.sha256(open(exe_cand, 'rb').read()).hexdigest(), 'programas': len(filas), 'filas': filas}


def comprobar(exe, tabla_path, sha_path, es_main):
    errores = []
    texto = open(tabla_path, encoding='utf-8').read()
    esperado_sha = open(sha_path, encoding='ascii').read().split()[0]
    if hashlib.sha256(texto.encode('utf-8')).hexdigest() != esperado_sha:
        errores.append('SHA-256 de la tabla distinto del registrado')
    viejo = json.loads(texto)
    progs = todos()
    n_prog, n_tabla = len(progs), len(viejo['filas'])
    if not (n_prog == n_tabla == viejo.get('programas') == 103):
        errores.append('numero de filas: programas=%d tabla=%d campo=%s (esperadas 103)' % (n_prog, n_tabla, viejo.get('programas')))
    ids_p, ids_t = [p[0] for p in progs], [f['id'] for f in viejo['filas']]
    if ids_p != ids_t:
        errores.append('ids distintos: solo en tabla %s; solo en programas %s' % (sorted(set(ids_t) - set(ids_p)), sorted(set(ids_p) - set(ids_t))))
    grupos = {}
    for f in viejo['filas']:
        grupos[f['grupo']] = grupos.get(f['grupo'], 0) + 1
    if grupos != {'conservacion': 51, 'diferencias_declaradas_de_las_54': 3, 'diferencias_declaradas': 28, GRUPO21: 21}:
        errores.append('grupos: %s' % grupos)
    por_id = {f['id']: f for f in viejo['filas']}
    for pid, grupo, perfil, src in progs:
        f = por_id.get(pid)
        if f is None:
            continue
        if f.get('fuente') != src or f.get('perfil') != perfil or f.get('grupo') != grupo or f.get('bytes') != len(src.encode('ascii')):
            errores.append('%s: difiere fuente/perfil/grupo/bytes' % pid)
            continue
        r = run(exe, src, perfil)
        if es_main and grupo != 'conservacion':
            esp = f.get('main')
        else:
            esp = {'outcome': f.get('outcome'), 'diagnosticos': f.get('diagnosticos')}
        if r != esp:
            errores.append('%s: difiere outcome/diagnosticos (%s)' % (pid, grupo))
        if grupo != 'conservacion' and not f.get('clase'):
            errores.append('%s: fila declarada sin clase' % pid)
    if os.path.exists(NOADJ_OUT):
        na = json.load(open(NOADJ_OUT, encoding='utf-8'))['filas']
        if [f['id'] for f in na] != [d['id'] for d in NOADJ]:
            errores.append('no adjudicables: ids distintos')
        for f in na:
            r = run(exe, f['fuente'], f['perfil'])
            if r != (f['main'] if es_main else f['candidato']):
                errores.append('%s: no adjudicable difiere' % f['id'])
    else:
        errores.append('falta el fichero de las 8 no adjudicables')
    return errores, viejo


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    tabla = OUT
    if '--tabla' in sys.argv:
        tabla = os.path.abspath(sys.argv[sys.argv.index('--tabla') + 1])
        args = [a for a in args if os.path.abspath(a) != tabla]
    sha = SHA if tabla == OUT else tabla + '.sha256'
    if '--regenerar' in sys.argv:
        t = regenerar(os.path.abspath(args[0]), os.path.abspath(args[1]))
        texto = json.dumps(t, indent=1, ensure_ascii=True, sort_keys=True) + '\n'
        open(OUT, 'w', encoding='utf-8', newline='\n').write(texto)
        open(SHA, 'w', encoding='ascii', newline='\n').write(hashlib.sha256(texto.encode('utf-8')).hexdigest() + '  match-conservacion-ext.json\n')
        print('programas', t['programas'])
        return
    # --check EXE [--main]: con --main el binario es `main` (las declaradas se comparan con el valor de main); sin el, el candidato
    errores, viejo = comprobar(os.path.abspath(args[0]), tabla, sha, '--main' in sys.argv)
    print('filas', len(viejo['filas']), 'diferencias:', errores or 'ninguna')
    sys.exit(1 if errores else 0)


main()
