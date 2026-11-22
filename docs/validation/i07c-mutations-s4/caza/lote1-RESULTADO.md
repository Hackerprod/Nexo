# Caza de programas discriminantes de I07c-S4

Base: `6394b67727334c479b700546d9d4b223da3c4943`. Blob `src/check/types.rs`: `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`.
Cada mutante se aplico al sitio literal unico del archivo archivado; se comprobo el SHA-256 antes de la aplicacion y despues de la restauracion.

Corpus congelado: 39127 fuentes unicas; SHA-256 `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`; semilla `64193`; SHA-256 del generador `7d00ac31cd38b2cfd4aec4c672c5b0d54799779d8f1996943840b10d6aa2a539`.
Se compararon exit y JSON completo de `check --format json`, y `run --format json` cuando alguno de los dos checks acepto. Maximo tres procesos de ejecucion. El perfil `f0-0005-b-v1` recorrio todo el corpus; `f0-0003-b-v1` y `f0-0002-b-v2`, las 3.040 fuentes dirigidas.
Las fuentes dirigidas cubren `var`/`let`, join/if, operadores `+ - * and or`, asignacion tardia, Index/Array, closure/match, ramas con retorno, TryAdd/AssertEq/Len y consumidores tras un operando fallido.
M00034, M00043 y M00044 ya tenian resultados completos al reanudar la ejecucion; se conservaron.

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

### M00008

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 63.9 s; caza: 582.1 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:2958`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00009

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 103.7 s; caza: 283.3 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): En `src/check/types.rs:2958-2959`, el padre recibe el bit `tainted` del hijo inmediatamente antes de `merge_child`. El unico efecto del argumento `true` frente a `false` es `padre.tainted |= hijo.tainted` (`src/check/types.rs:2321-2328`); tras esa asignacion, el OR es idempotente.

### M00010

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 57.0 s; caza: 344.3 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:3011`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00011

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 79.7 s; caza: 345.6 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): En `src/check/types.rs:3011-3012`, el padre recibe el bit `tainted` del hijo inmediatamente antes de `merge_child`. El unico efecto del argumento `true` frente a `false` es `padre.tainted |= hijo.tainted` (`src/check/types.rs:2321-2328`); tras esa asignacion, el OR es idempotente.

### M00018

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 53.3 s; caza: 293.4 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:7700`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00021

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 77.9 s; caza: 250.3 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:8210`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00025

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 61.6 s; caza: 256.6 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de invariante (independiente de la caza): En `src/check/types.rs:8950-8951`, el padre recibe el bit `tainted` del hijo inmediatamente antes de `merge_child`. El unico efecto del argumento `true` frente a `false` es `padre.tainted |= hijo.tainted` (`src/check/types.rs:2321-2328`); tras esa asignacion, el OR es idempotente.

### M00028

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 71.9 s; caza: 345.5 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9021`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00034

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 121.3 s; caza: 299.5 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9203`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00043

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 84.1 s; caza: 630.7 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9548`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

### M00044

**SIN_DIFERENCIA**. 45207 comparaciones fuente/perfil (f0-0005-b-v1: 39127, f0-0003-b-v1: 3040, f0-0002-b-v2: 3040). Compilacion: 88.5 s; caza: 274.7 s.

Semilla: `64193`. SHA-256 del corpus: `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`. Familias: directed, s4_family, s4_index, previous_corpus, rd_152, corpus_files y fixture_files; sus hashes estan en la tabla anterior. `s4_recheck` duplicaba fuentes de `s4_family`.

Sin programa tras la caza; no se afirma equivalencia.

Argumento de inalcanzabilidad para el cambio en `src/check/types.rs:9598`: ninguno demostrado a partir del codigo. El resultado empirico no lo sustituye.

