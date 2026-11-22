# I07c-S6-COR r3: recuento de diagnósticos al retirar las agrupaciones (solo hallazgo, filas de regresión y enmienda ya adoptada; sin cambios de código)

Base `1010938` y candidato `ddf551e`, binarios debug compilados de cada uno. Origen: D' (informe `C:\nexo-work\tmp\validador-dprima2-s6cor\informe.md`, motivo 1).

## 1. Hallazgo y causa raíz

`fn Main(){for x in ([]){}}` con `--profile f0-0003-b-v1` (`check` y `run`):

| | diagnósticos |
|---|---|
| base | 2: `NX-E0006` `types.check` `[19,23)` y `NX-E0103` `types.infer` `[20,22)` |
| candidato | 1: `NX-E0006` `types.check` `[20,22)` |
| control sin grupo `for x in []{}` (base y candidato) | 1: `NX-E0006` `[19,21)` |

Causa: al retirar la agrupación, el primario de `NX-E0006` (`for_iteration`) pasa de `([])` a `[]` y coincide con el sitio de `NX-E0103`; `Checker::record_error` (`src/check/types.rs`, `graph.emitted_sites`, §4.7 y §4.1.1: una sola emisión por sitio) descarta la segunda denuncia con el mismo primario. Es el mecanismo existente que ya iguala al control sin grupo; el recuento anterior dependía de que el grupo desplazara el primario.

## 2. Familia barrida

Barrido dirigido (`.scratch/dedupe_sweep.py`, no versionado; 420 programas, SHA-256 `34b8b4cd…`): 41 plantillas con huecos de `[]`, `[1]`, `[true]`, `1`, `G([])` en iterables de `for`, rangos, `let` con y sin anotación, operandos aritméticos y de igualdad, elementos de array, selectores, `Len`, argumentos, destinos de llamada, `match`, asignaciones, `Push`, `if`/`while`, `Some`/`Ok`, `AssertEq` y llamadas de closure, con 0 a 3 grupos y trivia (espacios y comentario de línea) dentro del grupo, perfiles `f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`, `check` y `run`, base frente a candidato.

**18 programas cambian de recuento, todos 2 -> 1 y todos iguales al control sin grupo** (el candidato iguala al control en los 18; 0 casos donde no):
- **Iterable de `for` con `([])`** (perfil J): `([])`, `(([]))`, `((([])))` y sus variantes con espacio y con comentario (9 programas; en F5 y v2 el `for` se rechaza antes por su token).
- **Destino de llamada `([])()`** (J, v2 y F5): `NX-E0111/non_callable_target` `[11,13)` en lugar de `[10,14)` más `NX-E0103` (9 programas, mismas variantes).

No hay otra familia en el barrido: operandos, argumentos, rangos, selectores, `match` y asignaciones no cambian de recuento. Los diferenciales de S6-COR (6.740 fuentes) ya pasaban con 0 diferencias de recuento porque el corpus no contenía `[]` agrupado en esas dos posiciones.

## 3. Enmienda del addendum (ya adoptada en main, `d40766d`)

> El número de diagnósticos es el del mismo programa sin agrupación: si al retirar las agrupaciones dos denuncias pasan a compartir primario, rige la deduplicación existente por sitio (una sola emisión por sitio, `record_error`) y el recuento iguala al del control sin grupo.

Ejemplo (`f0-0003-b-v1`, `check` y `run`): inválido `fn Main(){for x in ([]){}}` -> un diagnóstico `NX-E0006` `[20,22)`; igual que `fn Main(){for x in []{}}` (`[19,21)`). Con `(([]))` `[21,23)`; con `( [] )` `[21,23)`; con comentario dentro del grupo `[25,27)`. Destino de llamada: `fn Main(){([])();}` -> un diagnóstico `NX-E0111/non_callable_target` `[11,13)`, igual que `[]()` (`[10,12)`). No hay positivo con `[]` sin resolver; el positivo del mismo recorrido con tipo conocido es `fn Main(){let x=([1]);}` (fila existente).

## 4. Filas de regresión

`tests/i07c_group_primary.rs` (119 filas; `check` y `run`, exactamente un diagnóstico, sin stdout ni eventos): `dedupe_for_empty_array` (`[20,22)`), `_double` (`[21,23)`), `_space` (`[21,23)`), `_comment` (`[25,27)`) con control `for x in []{}` `[19,21)`; `dedupe_call_target_empty_array` (`[11,13)`), `_double` (`[12,14)`), `_comment` (`[16,18)`) y `_f5` (`[11,13)`) con control `[]()` `[10,12)`. Los esperados salen del control sin grupo (los desplazamientos de los paréntesis y de la trivia sobre el primario del control) y de la enmienda; no del binario del candidato. Sin positivo pareado (no existe programa válido con `[]` sin resolver); el control rechazado con la misma regla hace de pareja.

## 5. Mutación inversa

Mutante D1 (`docs/validation/i07c-s6cor/mutate.py`, copia desechable, `--profile default`, SHA-256 inicial, mutado y restaurado en `mutation-results.json`): `record_error` sin la deduplicación por sitio (`if false && !self.graph.emitted_sites.insert(..)`). MUERTO: fallan las filas de `i07c_group_primary` (la primera en `array_iterable_barrier_reports_the_inner_iterable`, después `call_binding_diagnostics_report_the_inner_target_and_argument`). Con la base compilada de `1010938` las mismas filas fallan por la causa inversa (dos diagnósticos).

## 6. Efectos laterales y alcance

Sin cambios de código ni de scripts. El recuento sigue siendo el del programa sin agrupación; los números de diagnóstico de los 18 programas cambian respecto de la base (2 -> 1) y coinciden con el control. R0001 y los demás sitios de lowering o ejecución con `span()` de una agrupación quedan para el corte S6-COR-2.
