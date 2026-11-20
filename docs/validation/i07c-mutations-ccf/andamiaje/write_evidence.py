import json, hashlib, collections, glob, shutil, os

OUT = 'D:/nexo-work/evidence'
M = 'D:/nexo-work/mut-s3'


def sh(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()


cls = json.load(open(OUT + '/clasificacion.json', encoding='utf-8'))
cnt = collections.Counter(c['clase'] for c in cls.values())
for d in ('no-terminacion', 'caza', 'andamiaje'):
    os.makedirs(OUT + '/' + d, exist_ok=True)
for fn in ('verify-hang.jsonl', 'verify-lib.jsonl', 'verify-nextest.jsonl'):
    shutil.copy(M + '/' + fn, OUT + '/no-terminacion/' + fn)
shutil.copy(M + '/verdict/resumen.jsonl', OUT + '/no-terminacion/mutant-verdict-resumen.jsonl')

hunt = []
for f in sorted(glob.glob(M + '/hunt-?.jsonl')) + [M + '/hunt-x.jsonl']:
    for line in open(f, encoding='utf-8'):
        r = json.loads(line)
        if 'id' in r:
            hunt.append(r)
with open(OUT + '/caza/hunt-resultados.jsonl', 'w', encoding='utf-8', newline='\n') as f:
    for r in hunt:
        f.write(json.dumps(r, ensure_ascii=False) + '\n')
shutil.copy(M + '/lote6-programas.json', OUT + '/caza/lote6-programas.json')
for fn in ('hunt.py', 'hunt_base.py', 'hunt_setup.py', 'gen_corpus.py', 'verify_hang.py', 'verify_lib.py',
           'verify_nextest.py', 'run_verdicts.py', 'build_evidence.py', 'write_evidence.py'):
    shutil.copy(M + '/' + fn, OUT + '/andamiaje/' + fn)
shutil.copy('D:/nexo-work/val-D1/h/fuzz.py', OUT + '/andamiaje/fuzz-generador-D.py')

corpus_sha = sh(M + '/hunt-corpus.json')
pop_sha = sh('D:/nexo-work/gate-d/docs/validation/i07c-mutations-ccf/population.json')
nt = [(i, c['evidencia']) for i, c in cls.items() if c['clase'] == 'detectada_por_no_terminacion']
disc = [r for r in hunt if r['resultado'] == 'DISCRIMINADO']
sd = [r for r in hunt if r['resultado'] == 'SIN_DIFERENCIA']

L = []
A = L.append
A('# Resultado de la mutación acotada de I07c')
A('')
A('Población congelada antes de ejecutar (`FREEZE.md`): 739 sustituciones de producción introducidas o '
  'modificadas por S1, S2, Bloque I y S3 respecto de `ccf6df1`; SHA-256 de `population.json` `%s`. '
  'Código evaluado: `11a70ec` (S3). Los literales de M00616 y M00617 están guardados con el carácter `§` '
  'mal codificado (`Â§`); la campaña los aplicó bien porque cada fragmento re-enumera la población en '
  'memoria, y la caza posterior tolera ese caso.' % pop_sha)
A('')
A('## Resumen por clase (los 739 mutantes, una sola clase cada uno)')
A('')
A('| Clase | Mutantes | Significado |')
A('|---|---:|---|')
desc = {
    'detectada': 'la campaña los detectó (prueba dirigida o suite completa) en todos los perfiles ejecutados',
    'detectada_por_regresion_nueva': 'sobrevivían a la campaña; una regresión añadida en los lotes 1 a 6 (solo pruebas) los mata y pasa con el código correcto',
    'detectada_por_no_terminacion': 'con el mutante un test no termina; con el código correcto termina en segundos (tabla más abajo)',
    'no_compila': 'el mutante no compila: sustitución inválida',
    'equivalente_con_argumento': 'argumento de invariante por mutante en los informes de los lotes; sin diferencia en el corpus de la caza',
    'sin_programa_tras_caza': 'ningún programa encontrado: ni los intentos dirigidos de los lotes ni 32.631 programas de la caza distinguen el mutante; NO se afirma equivalencia',
}
for k in ('detectada', 'detectada_por_regresion_nueva', 'detectada_por_no_terminacion', 'no_compila',
          'equivalente_con_argumento', 'sin_programa_tras_caza'):
    A('| `%s` | %d | %s |' % (k, cnt[k], desc[k]))
A('| **Total** | **%d** | |' % sum(cnt.values()))
A('')
A("Criterio de superviviente (AGENTS, «Rigor proporcional»): un superviviente solo bloquea si existe un "
  "programa o regresión que lo distinga; los %d `sin_programa_tras_caza` y los %d equivalentes se registran en "
  "`Pending.md`. Los supervivientes cuyo programa se encontró (incluido M00436, hallado por D') tienen su "
  "regresión." % (cnt['sin_programa_tras_caza'], cnt['equivalente_con_argumento']))
A('')
A('## Método')
A('- Campaña: `scripts/i07c_mutaciones.py --base ccf6df1` en 6 fragmentos (`I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, '
  'plazos ×3 y compilación sin información de depuración con `I07C_MUT_ENV_*`, andamiaje); los 100 primeros '
  'en release y debug, el resto en release; nivel dirigido y, para supervivientes, suite completa. '
  '`results.jsonl` une las 739 filas en el orden de la población.')
A('- Inconclusas por contención o por el cuelgue de salida de `nexo.exe` (defecto corregido en `main` 9be9f76): '
  'se repitieron sobre un árbol con la misma población (SHA idéntico) y el arreglo de salida aplicado.')
A('- Caza con generador: %d mutantes (los «sin programa» de los lotes 2 a 5); corpus de 32.631 programas únicos '
  '(SHA-256 `%s`): 40 semillas (101 a 140) × 900 del generador de D (`andamiaje/fuzz-generador-D.py`), los '
  'fuentes de `corpus/` y `tests/fixtures/`, las 152 filas RD y el programa de D\'. Se compara `check` (y `run` '
  'para `lower_match.rs`) del mutante contra el binario correcto, con parada tras 5 diferencias.'
  % (len(hunt), corpus_sha))
A('- Resultado de la caza: %d discriminados y %d sin diferencia. Los 10 discriminados (M00436, M00466, M00474, '
  'M00475, M00494, M00588, M00655, M00701, M00702, M00703) están en `caza/lote6-programas.json` con su programa '
  'y la salida de referencia, y su regresión está en `tests/i07c_solver_depth.rs` (lote 6).' % (len(disc), len(sd)))
A('')
A('## Detección por no terminación')
A('Con el código correcto: `check::tests` release 41,9 s y debug 6,0 s; `i07c_tests` debug 5,9 s y release 2,1 s; '
  'biblioteca completa debug 132,1 s; los cinco tests de integración implicados en M00676 y M00732 pasan en 0,013 a '
  '0,050 s. Con cada mutante el mismo filtro no termina en 300 s (biblioteca: 900 s; integración: 600 s por test).')
A('')
A('| Mutante | Evidencia |')
A('|---|---|')
for i, e in sorted(nt):
    txt = '; '.join('%s %s: %s' % (x[0], x[1], x[2]) for x in e)[:240].replace('|', '/')
    A('| %s | %s |' % (i, txt))
A('')
A('M00106 (debug) no es de esta clase: su filtro `i07c_tests` falla con el mutante tras 231,7 s y queda `detectada`. '
  'M00387, M00432 y M00538 dieron suite completa en verde con `mutant-verdict.py`; M00432 lo mata además un test '
  'del lote 3 y M00387 y M00538 se clasifican equivalentes con argumento (lote 5).')
A('')
A('## Ficheros')
for p in ('results.jsonl', 'clasificacion.json', 'no-terminacion/verify-hang.jsonl', 'no-terminacion/verify-lib.jsonl',
          'no-terminacion/verify-nextest.jsonl', 'no-terminacion/mutant-verdict-resumen.jsonl',
          'caza/hunt-resultados.jsonl', 'caza/lote6-programas.json'):
    A('- `%s` SHA-256 `%s`' % (p, sh(OUT + '/' + p)))
A('- `andamiaje/`: scripts usados (caza, verificaciones, construcción de esta evidencia); son andamiaje, no código de producción.')
open(OUT + '/RESULTADO.md', 'w', encoding='utf-8', newline='\n').write('\n'.join(L) + '\n')
print('RESULTADO.md escrito; clases', dict(cnt))
