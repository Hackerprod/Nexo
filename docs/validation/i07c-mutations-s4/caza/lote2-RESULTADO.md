# Caza de programas discriminantes I07c-S4, segundo lote

Base: `6394b67727334c479b700546d9d4b223da3c4943`; SHA-256 de `src/check/types.rs`: `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`.
Binario correcto de `gate.sh --cached-bin release`: `C:/nexo-work/cache/368f73fee16d6aea5b2db62d3e71ca0d00c85eba/585154a1ef365515/release.exe`; SHA-256 `68a07f5b6925b5a95db163fe092ce9a99256cbec1d049bb9078dc64129a019ab`.
Dos trabajadores: indices pares e impares de `s4-survivors-2.json`, arboles y targets separados; un mutante por arbol. Las 36 entradas tienen literal unico y sitio linea/columna comprobado contra el blob base. Parche exacto sobre blob verificado y restauracion SHA-256 tras cada mutante.

Corpus congelado: 39127 fuentes unicas; semilla `64193`; SHA-256 `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`; SHA-256 del generador `7d00ac31cd38b2cfd4aec4c672c5b0d54799779d8f1996943840b10d6aa2a539`.
Se compararon `exit` y JSON completo de `check --format json`; cuando alguno de los checks acepto, tambien `run --format json`. El perfil `f0-0005-b-v1` recorrio las 39.127 fuentes; `f0-0003-b-v1` y `f0-0002-b-v2`, las 3.040 dirigidas. A lo sumo dos ejecuciones de `nexo.exe` simultaneas entre ambos trabajadores.

Las familias dirigidas cubren `var`/`let`, join/if, `+ - * and or`, asignacion tardia, Index/Array, closure/match, ramas con retorno, TryAdd/AssertEq/Len y consumidores tras un operando fallido.

## Familias y hashes

| Familia | Fuentes | Nuevas unicas | SHA-256 |
|---|---:|---:|---|
| directed | 3040 | 3040 | `df9cffd9d9df53cf6d77a725cd1336420fac953611f3f5b140ac8de338e897c8` |
| s4_family | 3072 | 3072 | `854578a62fe37895cd8a98c7062ac2bd5fcd910e88c6035745ce92b720171758` |
| s4_index | 384 | 384 | `6c4071d6249a1c804d766bcd6bd0efdf18d60a95178d64ce21afc2c809d07433` |
| s4_recheck | 3072 | 0 | `854578a62fe37895cd8a98c7062ac2bd5fcd910e88c6035745ce92b720171758` |
| previous_corpus | 32631 | 32631 | `a6a0cf27028441f6938ab0ff71fd452a55406aa59bd04b3e7560d4676397a090` |
| rd_152 | 152 | 0 | `44f2538be8f934a90e4392e6a150178db8f607728ed3ab3e5167a28196b7968b` |
| corpus_files | 44 | 0 | `50a8a0774a900dd444f0b4331a48cb5bf3b4adcefcd983ca604e885beb8d2714` |
| fixture_files | 173 | 0 | `b4c94cb962892061fa6ba99d4de945fc3524059d00f50e4c1b743cc516801a7b` |

## Resultados

### M00048

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9611`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 26.9 s; caza: 742.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9611`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00053

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9635`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 76.0 s; caza: 755.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9635`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00055

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9647`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.2 s; caza: 376.6 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9647`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00056

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9664`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 29.2 s; caza: 379.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9664`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00059

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9682`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.2 s; caza: 380.8 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama devuelve `Ok(Type::Tainted)` y lo entrega a `check_against_context_one` (`src/check/types.rs:9675-9696`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00060

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9682`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.1 s; caza: 378.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama devuelve `Ok(Type::Tainted)` y lo entrega a `check_against_context_one` (`src/check/types.rs:9675-9696`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00062

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9696`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.8 s; caza: 411.0 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9696`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00063

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9717`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.2 s; caza: 418.7 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9717`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00072

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9806`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.3 s; caza: 384.2 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9806`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00074

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9885`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 32.5 s; caza: 369.8 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9885`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00076

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9893`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.0 s; caza: 367.6 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9893`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00077

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9893`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.7 s; caza: 369.6 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9893`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00078

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9945`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.8 s; caza: 401.7 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9945`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00079

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:9967`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.0 s; caza: 410.2 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9967`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00083

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:10004`, operador `false`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 29.1 s; caza: 456.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:10004`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00087

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:10047`, operador `false`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.7 s; caza: 463.4 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:10047`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00090

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:10050`, operador `false`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.9 s; caza: 415.1 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:10050`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00104

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:11788`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 32.7 s; caza: 388.0 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `known.invalid` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:11783-11790`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00105

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:11788`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.5 s; caza: 357.3 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `known.invalid` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:11783-11790`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00106

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:11840`, operador `or`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.7 s; caza: 357.7 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:11840`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00108

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12032`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.8 s; caza: 362.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `right_tainted` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12026-12034`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00109

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12032`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.6 s; caza: 363.0 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `right_tainted` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12026-12034`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00114

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12125`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.7 s; caza: 365.8 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La asignacion esta dentro de `if self.graph.fact_mut(visit).tainted`; el bit ya es `true` antes de escribirlo (`src/check/types.rs:12120-12128`).

### M00115

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12125`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.9 s; caza: 366.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La asignacion esta dentro de `if self.graph.fact_mut(visit).tainted`; el bit ya es `true` antes de escribirlo (`src/check/types.rs:12120-12128`).

### M00116

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12289`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.7 s; caza: 385.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `target_ty == Type::Tainted` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12284-12291`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00117

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12289`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 29.4 s; caza: 393.5 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama `target_ty == Type::Tainted` retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12284-12291`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00121

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12492`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.4 s; caza: 435.1 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:12492`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00122

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12713`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 76.7 s; caza: 735.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): `*invalid = true` en la misma rama (`src/check/types.rs:12708-12715`); `array_finish` devuelve `Ok(Type::Tainted)` si `invalid` (`src/check/types.rs:12721-12733`) y `check_against_context_one` marca el bit (`src/check/types.rs:11534-11557`).

### M00123

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12713`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 27.1 s; caza: 728.2 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): `*invalid = true` en la misma rama (`src/check/types.rs:12708-12715`); `array_finish` devuelve `Ok(Type::Tainted)` si `invalid` (`src/check/types.rs:12721-12733`) y `check_against_context_one` marca el bit (`src/check/types.rs:11534-11557`).

### M00124

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12827`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.9 s; caza: 356.3 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12815-12829`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00125

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:12827`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 31.1 s; caza: 355.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:12815-12829`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00127

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:13056`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.9 s; caza: 364.2 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:13042-13058`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00128

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:13056`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.7 s; caza: 364.8 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): La rama de error de comparacion retorna inmediatamente `Ok(Type::Tainted)` (`src/check/types.rs:13042-13058`). La asignacion se sigue de `Ok(Type::Tainted)` sin uso intermedio del bit. `check_against_context_one` lo marca antes de comparar con el contexto (`src/check/types.rs:11534-11557`).

### M00131

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:13698`, operador `false`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.6 s; caza: 337.9 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): `condition_tainted` guarda el bit del hijo antes de `merge_child`; despues `tainted |= condition_tainted || condition_result.is_err()` (`src/check/types.rs:13696-13701`). El cambio `merge_child(..., false/true)` solo agrega el mismo bit al padre mediante OR (`src/check/types.rs:2321-2334`), asi que el OR posterior es idempotente.

### M00137

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:14663`, operador `drop_assignment`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 30.9 s; caza: 364.0 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): Cuando `tainted && recover.is_none()` (`src/check/types.rs:14646-14663`), el `match` marca `state.invalid = true` o devuelve el error terminal de cuota (`src/check/types.rs:14665-14688`). El siguiente argumento crea un hijo con herencia `Empty` (`src/check/types.rs:10054-10078`); al terminar, `finish_assert_eq` aplica `tainted |= invalid` (`src/check/types.rs:14824`). La escritura temprana no llega a cambiar el bit final observable.

### M00138

**SIN_DIFERENCIA**. Sitio `src/check/types.rs:14663`, operador `true`. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 27.0 s; caza: 250.2 s.

Semilla `64193`; SHA-256 del corpus `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files (hashes arriba). `s4_recheck` repite `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): Cuando `tainted && recover.is_none()` (`src/check/types.rs:14646-14663`), el `match` marca `state.invalid = true` o devuelve el error terminal de cuota (`src/check/types.rs:14665-14688`). El siguiente argumento crea un hijo con herencia `Empty` (`src/check/types.rs:10054-10078`); al terminar, `finish_assert_eq` aplica `tainted |= invalid` (`src/check/types.rs:14824`). La escritura temprana no llega a cambiar el bit final observable.

