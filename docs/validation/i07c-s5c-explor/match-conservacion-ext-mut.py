#!/usr/bin/env python3
"""Siete mutaciones de la tabla extendida: el `--check` debe fallar (exit 1) con cada una; el control sin mutar debe dar exit 0.
Uso: python -B match-conservacion-ext-mut.py RUTA_CANDIDATO"""
import copy, hashlib, json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
CHECK = os.path.join(HERE, 'match-conservacion-ext.py')
exe = os.path.abspath(sys.argv[1])
tabla = json.load(open(os.path.join(HERE, 'match-conservacion-ext.json'), encoding='utf-8'))


def aplica(nombre, f):
    t = copy.deepcopy(tabla)
    f(t)
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, 'm.json')
        texto = json.dumps(t, indent=1, ensure_ascii=True, sort_keys=True) + '\n'
        open(p, 'w', encoding='utf-8', newline='\n').write(texto)
        open(p + '.sha256', 'w', encoding='ascii').write(hashlib.sha256(texto.encode('utf-8')).hexdigest() + '  m.json\n')  # sha coherente: solo falla por contenido
        r = subprocess.run([sys.executable, '-B', CHECK, '--check', exe, '--tabla', p], capture_output=True, text=True)
    print('%-34s exit=%d %s' % (nombre, r.returncode, r.stdout.strip()[:150]))
    return r.returncode


def fila(t, grupo, k=0):
    return [x for x in t['filas'] if x['grupo'] == grupo][k]


def m1(t): t['filas'].pop()                                           # fila de menos
def m2(t): t['filas'].append(dict(t['filas'][0], id='fila-de-mas'))    # fila de mas
def m3(t): fila(t, 'conservacion')['outcome'] = 'checked' if fila(t, 'conservacion')['outcome'] != 'checked' else 'rejected'   # outcome
def m4(t): fila(t, 'conservacion', 1)['diagnosticos'].append({'code': 'NX-E9999', 'subcause': None, 'primary': [0, 1], 'expected_type': None, 'found_type': None, 'relations': []})
def m5(t): fila(t, 'conservacion', 2)['perfil'] = 'f0-0005-b-v1'     # perfil cambiado
def m6(t): fila(t, 'diferencias_declaradas', 3)['diagnosticos'][0]['primary'] = [1, 2]   # fila del grupo declarado

def m7(t): fila(t, 'diferencias_declaradas_rmm3_fuera_2o_brazo', 2)['diagnosticos'][0]['primary'] = [1, 2]   # fila de las 21 de RM-M3


codigos = [aplica('control (sin mutar)', lambda t: None)]
for n, f in (('1 fila de menos', m1), ('2 fila de mas', m2), ('3 outcome alterado', m3), ('4 diagnostico anadido', m4), ('5 perfil cambiado', m5), ('6 fila declarada alterada', m6), ('7 fila RM-M3 alterada', m7)):
    codigos.append(aplica(n, f))
ok = codigos[0] == 0 and all(c == 1 for c in codigos[1:])
print('RESULTADO:', 'las 7 mutaciones fallan con exit 1 y el control pasa' if ok else 'FALLO DEL --check')
sys.exit(0 if ok else 1)
