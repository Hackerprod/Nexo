# I07c — Informe de exploración del rediseño

Estado: **exploración reabierta; sin PASS; ningún plan de Flujo B**. Base de código: `origin/main` `6e22181` para `src/check/types.rs`; los corpus históricos y el oráculo se leen del blob de `origin/i07c-estructural-r2`. El prototipo desechable está en `i07c-rediseno-proto` (`e97cce1`); sus cambios de `src/` no son entregables de integración. La parada anterior tras una regresión en t3584 fue prematura. Esta continuación analiza ese defecto, mide sondas adicionales y prepara el barrido por lotes.

## Continuación: mínimo, causa y sondas baratas

Se redujo t3584 a 89 bytes, ASCII sin BOM ni LF final, perfil `f0-0002-b-v2`:

```nexo
fn Main(){var q=[];var r=[];(if true then [q] else (if true then [1] else q));r=q;q=[1];}
```

El oráculo S5c y el candidato 5 dan solo `NX-E1303` `[65,68)`; `txmarks` añade `NX-E0102/type_mismatch` `[85,86)`. `r=q` es necesario para reproducirlo en esta reducción: al retirarlo, la sonda deja de añadir el error. El rastro de planning de `txmarks` muestra `read_plan` para `q` y `r`, y la primera igualdad de unión pasa de pendiente con `atomic0` a `Err` inmediato en la pasada final. La marca reconstruida conserva la obligación, pero su alcance de `if` interior no cubre la lectura `q` del ancla exterior. La hipótesis causal es que `read_plan` aplica prematuramente el tipo de `q=[1]` a esa lectura; está pendiente falsarla con variantes de alcance.

Se ejecutaron las 45 filas del oráculo histórico y doce sondas dirigidas con el mismo binario release anterior a las nuevas variantes (los cinco mínimos históricos, este mínimo reducido, las dos formas de `planned_projection`, `incompatible_prefix`, `anchored_first_branch`, el negativo recursivo y el positivo). Contra ese oráculo anterior a T5/N, `candidate5` y `candidate5+txunify` dan 45/45 y 10/12: fallan ambas proyecciones. `candidate7` (sin `atomic0`), `txmarks` y `txmarks+txunify` dan 40/45 y 5/12: las filas rojas son W0, B_qr, P1, c3 y w1; fallan los cinco mínimos históricos, el mínimo reducido e `incompatible_prefix`. Las dos formas de `planned_projection` sí pasan con estas tres variantes. La sonda de `anchored_first_branch` usa el esperado adjudicado por B, solo `NX-E0102` `[136,137)`. Este recuento es dirigido, no sustituye las tablas A/B/C/D del barrido.

El oráculo desechable publicado en `i07c-rediseno-proto` incorpora ahora T5 y la regla N del literal: conserva las obligaciones internas de las ramas, pero una `if` con condición fallida y un literal con elemento erróneo no ofrecen resultado al consumidor. Pasan los 17 ejemplos normativos y controles del propio oráculo, incluidos los dos nuevos transcritos del ADR. En las 45 filas, O08 cambia por T5/N: se retiran el `NX-E1303` `[137,138)` y el `NX-E0102` derivado `[155,158)` del esperado anterior. Frente a esta lectura corregida, `candidate5`/`txunify` dan **44/45** y `candidate7`/`txmarks`/`txmarks+txunify` **39/45**. Esta derivación de O08 requiere revisión independiente antes de ser oráculo de cierre; se conserva el esperado previo para auditoría.

Las variantes direccionales y de alcance (`txmarksright`, `txmarksleft`, `txmarksstatement`, `txmarksoccurrence` y combinaciones) se publicaron primero pendientes de compilar bajo el cerrojo de la máquina compartida; sus resultados aparecen a continuación. El arnés versionado `scripts/i07c_rd_cheap.py` reproduce las sondas; `scripts/i07c_rd_sweep.py` conserva resultados por lotes para el barrido.

### Resultado posterior de las variantes de alcance (release, sondas baratas)

La compilación terminó bajo `heavy-lock`. La tabla usa el oráculo T5/N corregido y **trece** sondas: las doce anteriores y el mínimo W0 de 78 bytes. Todas se repitieron con el mismo binario release `rd-proto-v2.exe`; los resultados por programa quedaron en `.scratch/cheap-*.json` del worktree desechable. La columna A compara solo donde el binario histórico `main` `eeb3c9c` coincidía con el esperado; el blob de `src/check/types.rs` de esa base es idéntico al de `origin/main` actual, pero aún falta compilar el `origin/main` íntegro para ratificar A. Las filas W0, B_qr, P1, c3 y w1 ya fallan en ese `main`: restaurar su salida es **pérdida de arreglos**, no regresión A. O08 falla en `main` y en todas las variantes bajo la lectura T5/N.

| variante añadida a candidato 5 | matriz T5/N | sondas | A matriz / sondas frente a `eeb3c9c` | causa observada |
|---|---:|---:|---:|---|
| ninguna (`atomic0`) | 44/45 | 11/13 | 0 / 2 | `planned_projection` sigue dando `NX-E0102` por siembra temprana. |
| `txunify` | 44/45 | 11/13 | 0 / 2 | Snapshot de `Err` no cambia una obligación suspendida. |
| `txmarks` | 39/45 | 5/13 | 0 / 6 | Reconstruir todas las marcas deja `read_plan` prematuro en los seis mínimos t y pierde los cinco arreglos W0/P1. |
| `txmarksright` | 39/45 | 11/13 | 0 / 0 | Quita el error de t, pero la marca de `q` impide la concreción de `r` en W0 y análogos. |
| `txmarksleft` | 44/45 | 5/13 | 0 / 8 | No restaura la marca del operando pendiente derecho de `planned_projection`; conserva los errores t. |
| `txmarksconcrete` | 39/45 | 11/13 | 0 / 0 | Misma pérdida W0/P1 que `txmarksright`; exigir lado izquierdo concreto no decide el alcance de siembra. |
| `txmarksstatement` | 39/45 | 11/13 | 0 / 0 | Ampliar el alcance elimina el error t, pero la marca global sigue suprimiendo el plan concreto de `r`. |
| `txmarksoccurrence` | 39/45 | 11/13 | 0 / 0 | Mismo efecto: el alcance evita retipado de la lectura anterior, pero no separa evidencia de cierre y contexto. |
| `txmarksstatement,txmarksright` | 39/45 | 11/13 | 0 / 0 | Combina dos filtros que dejan idéntico el fallo W0/P1. |
| `txmarksoccurrence,txmarksright` | 39/45 | 11/13 | 0 / 0 | Igual que la fila anterior. |
| `txmarksoccurrence,txunify` | 39/45 | 11/13 | 0 / 0 | El snapshot de `Err` no resuelve las obligaciones suspendidas. |
| `txmarksproj` | 44/45 | **13/13** | **0 / 0** | Reconstruye la marca solo cuando la pasada tiene una proyección pendiente; corrige las dos proyecciones sin alterar los mínimos t y W0. O08 sigue fallando por T5/N. El criterio global es provisional: la marca se filtra por presencia de proyección en la función, no por identidad de obligación. |

Un segundo mínimo de 78 bytes aísla la pérdida de arreglo: `fn Main(){var r=[];var q=[];(if true then true else q);[r,q];q=[[true]];r=[];}`. Oráculo y candidato 5: solo `NX-E1303` `[52,53)`; `txmarksright` añade `NX-E0103` `[16,18)`, igual que `main`. En W0, el plan de candidato 5 tiene `concrete_plan` para `r` y `q` y ningún `read_plan`; `txmarksright` invierte esos dos conjuntos. El contraste final pasa de incompatible inmediatamente a obligación pendiente y el `[]` inicial de `r` no se resuelve. La causa común de estas variantes es que `deferred_join` se usa simultáneamente para decidir la disponibilidad de contexto de una lectura anterior y para impedir completar constructores al cierre; la regla del ADR exige conservar las igualdades compatibles de `[r,q]` y `q=[[true]]` aunque la unión `true/q` falle (§4.1.2.2 y 4.1.2.4). Las variantes con A=0 en estas sondas pasan al barrido completo, aun cuando todavía no cumplan 45/45.

La comprobación dirigida de las 103 filas de `match-conservacion-ext.py --check` con binarios release conserva las **51 filas de conservación** en las ocho variantes con A=0; `candidate5` reproduce la tabla completa. Las diferencias restantes son solo de filas declaradas: `txmarksconcrete` 0; `txmarksright` 2; `txmarksstatement` 2; `txmarksoccurrence` 3; `txmarksstatement,txmarksright` 1; `txmarksoccurrence,txmarksright` 1; `txmarksoccurrence,txunify` 3; `txmarksproj` 1 (`dif-s9908255-3958-anadido`). Los identificadores y salidas textuales están en `.scratch/match-*.txt` del worktree desechable. Estas diferencias requieren adjudicación por cláusula; la conservación de 51 no prueba las otras puertas de §3.3.

### Contraejemplo nuevo de `txmarksproj` antes del barrido completo

El mínimo de 115 bytes `fn Main(){var s=[];let b=s[0];var q=[];var r=[];(if true then [q] else (if true then [1] else q));r=q;q=[1];s=[1];}` fue ejecutado con `check --format json --profile f0-0002-b-v2` en los binarios release de `main` (SHA-256 `44bef37452e4…`) y del prototipo (`57f9db1da0d2…`). `main` y candidato 5 dan solo `NX-E1303` `[85,88)`; `txmarksproj` añade `NX-E0102/type_mismatch` `[105,106)`. Al quitar la proyección independiente de `s`, los tres dan solo el `NX-E1303` `[65,68)`. Es regresión de núcleo dirigida: el criterio `has_projection_items` mira toda la función y reconstruye la marca de una obligación de `q` que no depende de `s[0]`. El barrido de corpus sigue siendo necesario para medir A/B/C/D y buscar otras causas; el ejemplo ya impide llamar PASS a la variante global.

## Evidencia directa disponible de la primera exploración

Compilé el prototipo de candidato 5 con una sonda `txunify` que hace atómica la llamada completa al solucionador mediante una copia de `Checker`. Es una prueba de arquitectura, no una representación de producción. Con `check --format json --profile f0-0002-b-v2` sobre fuentes ASCII sin BOM ni LF final:

| variante | t3584: diagnóstico extra `[146,148)` | `planned_min`: código en `q` `[69,70)` | R4 | negativo recursivo | positivo |
|---|---|---|---|---|---|
| candidato 5 (`atomic0`) | no | `NX-E0102` (incorrecto; esperado `NX-E1303`) | solo `NX-E1303` `[94,95)` | `NX-E0103` `[66,67)` | aceptado |
| candidato 5 + `txunify` | no | `NX-E0102` (incorrecto) | igual | igual | aceptado |
| candidato 7 (sin `atomic0`) | sí, `NX-E0102` (incorrecto) | `NX-E1303` (correcto) | igual | igual | aceptado |
| candidato 7 + `txunify` | sí, `NX-E0102` (incorrecto) | `NX-E1303` (correcto) | igual | igual | aceptado |
| candidato 5 + `txmarks` | sí, `NX-E0102` (incorrecto) | `NX-E1303` (correcto) | igual | igual | aceptado |
| candidato 5 + `txmarks` + `txunify` | sí, `NX-E0102` (incorrecto) | `NX-E1303` (correcto) | igual | igual | aceptado |

`txunify` **no altera** las seis firmas dirigidas: la transacción de la llamada al solucionador, por sí sola, no reemplaza `atomic0`. `txmarks` sí cambia el código causal en ambos `planned_projection` y elimina en el mínimo los `NX-E0102` derivados `[83,84)` y `[90,91)`, pero vuelve a introducir el `NX-E0102` causalmente espurio de t3584. El resultado es idéntico con `txunify` añadido. Las salidas completas de `planned_min` y `planned_full` conservan otras diferencias; el cuadro destaca el sitio decisivo. Ninguna variante medida satisface simultáneamente los dos mínimos. Estos números son medición propia de este encargo; las cifras de corpus de INFORME-1..8 siguen siendo históricas.

El trazado `dbg` del mismo binario localiza el desvío antes de la pasada final. En `planned_min`, la igualdad `Bool` frente a `q` registra una obligación diferida (`branch=true`, constructor 1) durante planning. Con `atomic0`, la pasada final entra en ambas uniones con `expected=true` y emite `NX-E0102` en el `q` de la primera; sin `atomic0`, entra con `expected=false` y emite `NX-E1303`. La lectura del código muestra que `atomic0` restaura todos los `type_nodes` previos pero **conserva la nueva `PendingDeferredJoin`**; con ella pierde `deferred_join` y `branch_join_scopes`, que `collect_planned_shapes()` consulta para autorizar siembra y `read_plan`. Esto identifica un estado incoherente entre obligación y marcas. Reconstruirlas desde la obligación corrige `planned_projection`, pero es insuficiente: hace reaparecer la ligadura que produce el error de t3584. Haría falta una representación transaccional que resuelva ambas consecuencias de la misma obligación; esta ronda no la demostró.

## Corpus congelados y reproducibilidad

Los SHA-256 de los blobs históricos comprobados con `git show` son: c1 `e78702d1…42742d7cfe9`, c2 `514c929f…f5584427cd864`, dirigido5 `684d6234…5dd5e09`, corpus-b `83f9e726…e410af1f`, perm `8997f939…700cf72`, corpus-t `a1c740bd…ffce2c5`. Los valores completos de los tres primeros están en el brief §3.1 y los demás se recomputan con `git show origin/i07c-estructural-r2:<ruta> | sha256sum` (sobre bytes del blob, no sobre archivo de trabajo Windows).

Dos semillas nuevas de 8.000 fuentes cada una se congelaron **antes** de medir la variante `txmarks`, en `docs/validation/i07c-rediseno/` (`424ffda`), junto con sus SHA. Las tres muestras B de 4.500 se recuperaron y congelaron en el mismo directorio (`69bf399`). La rama histórica no publica sus SHA, por lo que la identidad byte a byte con aquella exploración no está demostrada.

Comandos de la sonda, desde el worktree desechable, con `CARGO_TARGET_DIR=C:/nexo-work/target-rd` y toolchain 1.99.1:

```text
python -B scripts/heavy-lock.py run --session i07c-rediseno -- cargo build --release --offline
python -B scripts/i07c_rd_probe.py C:/nexo-work/target-rd/release/nexo.exe
python -B scripts/i07c_rd_probe.py C:/nexo-work/target-rd/release/nexo.exe --flags pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr,skipv3,txunify
```

Para reproducir `txmarks`, añadir `,txmarks` a la lista de `--flags` anterior. Para la combinación con `txunify`, añadir además `,txunify`. La compilación de ambas sondas pasó con rustc 1.99.1 bajo `scripts/heavy-lock.py`; el cerrojo estuvo ocupado por la campaña `i07c-mut-regiones-c` y se reintentó con pausas de 60 segundos, sin tocar el archivo ni procesos ajenos.

## Resultado por corpus y criterio de parada

| conjunto del brief §3.1 | regresiones de core de `txmarks` frente a `main` | observación |
|---|---:|---|
| c1 (24.000) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| c2 (16.000) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| corpus-b (4.000) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| perm (4.044) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| corpus-t (10.584) | **NO EVALUADO** en conjunto | Un mínimo propio, t3584, presenta la regresión detallada abajo. |
| B 7021993 (4.500) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| B 4417061 (4.500) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| B 9908255 (4.500) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| dirigido5 (6.000) | **NO EVALUADO** | Sin tabla A/B/C/D. |
| c1-nueva (8.000) | **NO EVALUADO** | Congelado con SHA antes de medir `txmarks`. |
| c2-nueva (8.000) | **NO EVALUADO** | Congelado con SHA antes de medir `txmarks`. |
| mínimo t3584 (una fuente histórica de corpus-t) | **1 programa con regresión de core** | `main` y oráculo: `NX-E0103` `[85,87)` + `NX-E1303` `[126,128)`; `txmarks` añade `NX-E0102/type_mismatch` `[146,148)`. |

El fallo dirigido falsó `txmarks`; la continuación mide otras variantes. No se ha lanzado aún el barrido masivo ni hay tabla A/B/C/D por corpus, ni afirmación de 0 global. Los ceros del candidato 5 y las 56/1.007 de variantes históricas son datos de INFORME-1..8, no resultados propios de `txmarks`.

## Puertas del brief §7

| pieza | estado |
|---|---|
| Regla única, ejemplos, citas | **Hipótesis entregada** en `I07c-rediseno-restauracion-atomica-diseno.md`; falta demostración. El negativo T5 se derivó de `F0-0002-lagunas.md` en `2dfcf22`. |
| Prototipo desechable y comandos | **Entregado y falsado** en `i07c-rediseno-proto`; `txmarks` corrige `planned_projection` pero regresa en t3584, y `txunify` no cambia la decisión. |
| Barrido A/B/C/D de todos los corpus; matriz 45; tablas de `match`; aceptación y debug/release | **NO EVALUADO aún para `txmarks`**; el barrido por lotes está preparado para las siete variantes con A=0 en sondas baratas. No se importan como propios los ceros del candidato 5: ese candidato falla `planned_projection`. |
| 152 RD, objetivo, 3.674, twin, fuzz, pila, pasadas, tiempo | **NO EVALUADO aún para `txmarks`**. La compilación release de la sonda sí pasó; no equivale a puerta completa. |
| 17 tests de §3.6 y cada fila abierta de la lista de defectos | **NO EVALUADO para `txmarks`**; siguen nominalmente abiertos hasta un nuevo diseño y derivación individual. No se editó ningún test. |

El corpus base sigue limitado a una función `Main`, seis hojas y profundidad tres; el oráculo histórico no modela `match`, `while`, bloques `{}` ni `AssertEq`, y precede a T5. Ni el recuento de muestras ni un positivo dirigido prueban ausencia de regresiones. La exploración continúa y **no se escribe plan de implementación**. Quedan pendientes las lagunas y piezas S5a.2/S5a.3/S5a.5, `match`, los 17 tests y las 50 filas abiertas; no se reclasifican como deuda sin programa.
