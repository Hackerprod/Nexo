"""Merge two completed S4b worker records into the requested deliverables."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SURVIVORS = json.loads(Path('D:/nexo-work/mut-s3/s4-survivors-2.json').read_text(encoding='utf-8'))
FREEZE = json.loads((ROOT / 'corpus-freeze.json').read_text(encoding='utf-8'))
assert hashlib.sha256((ROOT / 'corpus-frozen.json').read_bytes()).hexdigest() == FREEZE['sha256']

rows = [json.loads(line) for worker in (0, 1)
        for line in (ROOT / f'hunt-resultados-w{worker}.jsonl').read_text(encoding='utf-8').splitlines()
        if line.strip()]
by_id = {row['id']: row for row in rows}
assert len(by_id) == len(rows) == len(SURVIVORS), 'unfinished or duplicate worker result'
assert set(by_id) == {m['id'] for m in SURVIVORS}
programs = [item for worker in (0, 1)
            for item in json.loads((ROOT / f'programas-discriminantes-w{worker}.json').read_text(encoding='utf-8'))]
programs_by_id = {}
for item in programs:
    programs_by_id.setdefault(item['id'], []).append(item)

TAINTED_RESULT = ('La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. '
                  '`check_against_context_one` lo marca antes de comparar con el contexto '
                  '(`src/check/types.rs:11534-11557`).')
invariants = {
    'M00059': 'La rama devuelve `Ok(Type::Tainted)` y lo entrega a `check_against_context_one` '
              '(`src/check/types.rs:9675-9696`). ' + TAINTED_RESULT,
    'M00060': 'La rama devuelve `Ok(Type::Tainted)` y lo entrega a `check_against_context_one` '
              '(`src/check/types.rs:9675-9696`). ' + TAINTED_RESULT,
    'M00104': 'La rama `known.invalid` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:11783-11790`). ' + TAINTED_RESULT,
    'M00105': 'La rama `known.invalid` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:11783-11790`). ' + TAINTED_RESULT,
    'M00108': 'La rama `right_tainted` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12026-12034`). ' + TAINTED_RESULT,
    'M00109': 'La rama `right_tainted` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12026-12034`). ' + TAINTED_RESULT,
    'M00114': 'La asignacion esta dentro de `if self.graph.fact_mut(visit).tainted`; '
              'el bit ya es `true` antes de escribirlo (`src/check/types.rs:12120-12128`).',
    'M00115': 'La asignacion esta dentro de `if self.graph.fact_mut(visit).tainted`; '
              'el bit ya es `true` antes de escribirlo (`src/check/types.rs:12120-12128`).',
    'M00116': 'La rama `target_ty == Type::Tainted` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12284-12291`). ' + TAINTED_RESULT,
    'M00117': 'La rama `target_ty == Type::Tainted` retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12284-12291`). ' + TAINTED_RESULT,
    'M00122': '`*invalid = true` en la misma rama (`src/check/types.rs:12708-12715`); '
              '`array_finish` devuelve `Ok(Type::Tainted)` si `invalid` '
              '(`src/check/types.rs:12721-12733`) y `check_against_context_one` marca el bit '
              '(`src/check/types.rs:11534-11557`).',
    'M00123': '`*invalid = true` en la misma rama (`src/check/types.rs:12708-12715`); '
              '`array_finish` devuelve `Ok(Type::Tainted)` si `invalid` '
              '(`src/check/types.rs:12721-12733`) y `check_against_context_one` marca el bit '
              '(`src/check/types.rs:11534-11557`).',
    'M00124': 'La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12815-12829`). ' + TAINTED_RESULT,
    'M00125': 'La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:12815-12829`). ' + TAINTED_RESULT,
    'M00127': 'La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:13042-13058`). ' + TAINTED_RESULT,
    'M00128': 'La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` '
              '(`src/check/types.rs:13042-13058`). ' + TAINTED_RESULT,
    'M00131': '`condition_tainted` guarda el bit del hijo antes de `merge_child`; '
              'despues `tainted |= condition_tainted || condition_result.is_err()` '
              '(`src/check/types.rs:13696-13701`). El cambio `merge_child(..., false/true)` '
              'solo agrega el mismo bit al padre mediante OR '
              '(`src/check/types.rs:2321-2334`), asi que el OR posterior es idempotente.',
    'M00137': 'Cuando `tainted && recover.is_none()` (`src/check/types.rs:14646-14663`), '
              'el `match` marca `state.invalid = true` o devuelve el error terminal de cuota '
              '(`src/check/types.rs:14665-14688`). El siguiente argumento crea un hijo con '
              'herencia `Empty` (`src/check/types.rs:10054-10078`); al terminar, '
              '`finish_assert_eq` aplica `tainted |= invalid` (`src/check/types.rs:14824`). '
              'La escritura temprana no llega a cambiar el bit final observable.',
    'M00138': 'Cuando `tainted && recover.is_none()` (`src/check/types.rs:14646-14663`), '
              'el `match` marca `state.invalid = true` o devuelve el error terminal de cuota '
              '(`src/check/types.rs:14665-14688`). El siguiente argumento crea un hijo con '
              'herencia `Empty` (`src/check/types.rs:10054-10078`); al terminar, '
              '`finish_assert_eq` aplica `tainted |= invalid` (`src/check/types.rs:14824`). '
              'La escritura temprana no llega a cambiar el bit final observable.',
}

ordered_rows = []
for m in SURVIVORS:
    row = by_id[m['id']].copy()
    row['invariant_argument'] = invariants.get(m['id'])
    ordered_rows.append(row)
(ROOT / 'hunt-resultados.jsonl').write_text(
    ''.join(json.dumps(row, ensure_ascii=False) + '\n' for row in ordered_rows), encoding='utf-8')
(ROOT / 'programas-discriminantes.json').write_text(
    json.dumps(programs, ensure_ascii=False, indent=2), encoding='utf-8')

lines = [
    '# Caza de programas discriminantes I07c-S4, segundo lote', '',
    'Base: `6394b67727334c479b700546d9d4b223da3c4943`; '
    'SHA-256 de `src/check/types.rs`: `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`.',
    'Binario correcto de `gate.sh --cached-bin release`: '
    '`C:/nexo-work/cache/368f73fee16d6aea5b2db62d3e71ca0d00c85eba/585154a1ef365515/release.exe`; '
    'SHA-256 `68a07f5b6925b5a95db163fe092ce9a99256cbec1d049bb9078dc64129a019ab`.',
    'Dos trabajadores: indices pares e impares de `s4-survivors-2.json`, arboles y targets separados; '
    'un mutante por arbol. Las 36 entradas tienen literal unico y sitio linea/columna comprobado '
    'contra el blob base. Parche exacto sobre blob verificado y restauracion SHA-256 tras cada mutante.', '',
    f"Corpus congelado: {FREEZE['entries']} fuentes unicas; semilla `{FREEZE['seed']}`; "
    f"SHA-256 `{FREEZE['sha256']}`; SHA-256 del generador `{FREEZE['generator_sha256']}`.",
    'Se compararon `exit` y JSON completo de `check --format json`; cuando alguno de los checks acepto, '
    'tambien `run --format json`. El perfil `f0-0005-b-v1` recorrio las 39.127 fuentes; '
    '`f0-0003-b-v1` y `f0-0002-b-v2`, las 3.040 dirigidas. A lo sumo dos ejecuciones '
    'de `nexo.exe` simultaneas entre ambos trabajadores.', '',
    'Las familias dirigidas cubren `var`/`let`, join/if, `+ - * and or`, asignacion tardia, '
    'Index/Array, closure/match, ramas con retorno, TryAdd/AssertEq/Len y consumidores '
    'tras un operando fallido.', '',
    '## Familias y hashes', '',
    '| Familia | Fuentes | Nuevas unicas | SHA-256 |', '|---|---:|---:|---|',
]
for group in FREEZE['groups']:
    lines.append(f"| {group['family']} | {group['count']} | {group['new_unique']} | `{group['sha256']}` |")
lines += ['', '## Resultados', '']
for m in SURVIVORS:
    mid = m['id']
    row = by_id[mid]
    counts = ', '.join(f'{profile}: {count}' for profile, count in row['counts'].items())
    lines += [f'### {mid}', '',
              f"**{row['status']}**. Sitio `src/check/types.rs:{m['line']}`, operador `{m['operator']}`. "
              f"{row['total_comparisons']} comparaciones fuente/perfil ({counts}). "
              f"Compilacion: {row['build_seconds']} s; caza: {row['hunt_seconds']} s.", '',
              f"Semilla `{row['seed']}`; SHA-256 del corpus `{row['corpus_sha256']}`. "
              'Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files '
              'y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.', '']
    if row['status'] == 'DISCRIMINADO':
        items = programs_by_id.get(mid, [])
        assert items and len(items) == row['differences'], mid
        for number, item in enumerate(items, 1):
            path = ROOT / f'{mid}-{number}.n'
            path.write_text(item['source'], encoding='utf-8', newline='')
            lines += [f"Diferencia {number}: `{path.name}`; perfil `{item['profile']}`; "
                      f"modo `{item['mode']}`; familia `{item['family']}`.", '',
                      '```nexo', item['source'], '```', '', 'Salida correcta:', '',
                      '```json', json.dumps(item['correct'], ensure_ascii=False, indent=2), '```', '',
                      'Salida del mutante:', '', '```json',
                      json.dumps(item['mutant'], ensure_ascii=False, indent=2), '```', '']
    else:
        assert row['status'] == 'SIN_DIFERENCIA' and not programs_by_id.get(mid)
        assert row['counts']['f0-0005-b-v1'] == FREEZE['entries']
        assert all(row['counts'][p] == 3040 for p in ('f0-0003-b-v1', 'f0-0002-b-v2'))
        lines += ['Sin programa tras la caza; no se afirma equivalencia.', '']
    if mid in invariants:
        lines += ['Argumento de invariante (independiente de la caza): ' + invariants[mid], '']
    else:
        lines += [f'Argumento de inalcanzabilidad para el cambio en '
                  f'`src/check/types.rs:{m["line"]}`: ninguno demostrado a partir del codigo. '
                  'El resultado empirico no lo sustituye.', '']

(ROOT / 'RESULTADO.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')
print(f'Resultados: {len(rows)} mutantes; {len(programs)} diferencias; '
      f'{sum(row["total_comparisons"] for row in rows)} comparaciones fuente/perfil')
