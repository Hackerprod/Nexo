# I07c — Informe de exploración del rediseño

Estado: **continúa el rediseño de representación; sin PASS; ningún plan de Flujo B**. La «PARADA DEMOSTRADA» de la continuación 3 se refiere solamente a la familia de marcas sobre `TypeNode`; no demuestra inviabilidad de la regla del ADR. Base de código: `origin/main` `6e22181` para `src/check/types.rs`; los corpus históricos y el oráculo se leen del blob de `origin/i07c-estructural-r2`. El prototipo desechable y los resúmenes publicados están en `i07c-rediseno-proto`; sus cambios de `src/` no son entregables de integración. La continuación 3 ejecutó el barrido completo de firmas, midió quince variantes con causa y detuvo aquella familia al reaparecer regresiones estructurales en corpus nuevos.

## Continuación 4: representación R1 (en curso)

El diseño actualizado en `I07c-rediseno-restauracion-atomica-diseno.md` separa contexto al entrar en el nodo, obligación suspendida y completación posterior; añade libro de igualdad por ocurrencia, registro de ancla, testigo parcial y deber de resolución por constructor. Esto es una hipótesis de arquitectura, no un PASS.

La primera compilación release del prototipo R1 pasó bajo `heavy-lock` con rustc 1.99.1 y `CARGO_BUILD_JOBS=4`. Su primera sonda, **antes** de conservar evidencia tras retirar la obligación, dio 14/16 mínimos y 44/45 filas T5/N: fallaron ambas formas de `planned_projection` y O08. El fallo de las proyecciones tiene una causa de ciclo de vida comprobada leyendo `function()`: al salir de la función se drena la cola de obligaciones antes de `collect_planned_shapes()`, por lo que esa cola no puede servir también de historial de contexto. O08 conserva los cuatro C propios de las condiciones, pero añade `NX-E1303` `[137,138)` y `NX-E0102` `[155,158)`; T5 exige contaminar el resultado de cada `if` con condición fallida, de modo que no ofrezca ancla a la unión ni tipo al array exterior.

La repetición con historial separado se midió con el mismo binario SHA registrado en `docs/validation/i07c-rediseno/r1-sondas.json` de la rama desechable (commit `496d71f`). La tabla siguiente es **puerta dirigida**, no barrido masivo ni PASS:

| configuración R1 | mínimos correctos / 16 | matriz T5/N / 45 | causa falsadora |
|---|---:|---:|---|
| obligación activa | 14 | 44 | La obligación se drena antes de planificar; `planned_projection` se decide con contexto adelantado. |
| historial por raíz | 6 | 39 | Corrige `planned_projection`, pero reintroduce t2192/t3584/t3594/t3596/t7094, 78/97/115 B y W0/B_qr/P1/c3/w1. La raíz compartida no representa qué lectura vio cada prefijo. |
| historial por raíz + T5 inicial | 3 | 39 | O08 pierde sus dos errores derivados, pero también pierde los C independientes `[100,101)` y `[107,109)` porque el prototipo retiró el contexto heredado de los brazos; además fallan `planned_projection` e `incompatible_prefix`. |

R1 queda **FALLA**. La corrección de T5 conserva el contexto que ya estaba distribuido antes de comprobar la condición; solo contamina el resultado de la `if` para consumidores.

R2 usa instantáneas del prefijo en la primera visita de cada lectura y nodos sombra para las lecturas que no pueden ver la siembra posterior. Compiló y se midió bajo el mismo cerrojo; `docs/validation/i07c-rediseno/r2-sondas.json` en `i07c-rediseno-proto` (`ac71c04`) conserva SHA del binario, flags y salidas. La variante sin T5 dio **10/16** mínimos y **36/45** filas; con T5, **10/16** y **37/45**. El mínimo de 115 B se conserva y t3584 no regresa, pero W0 (78 B) cambia la `NX-E1303` `[52,53)` por `NX-E0102/not_indexable` en el mismo sitio; el de 97 B añade `NX-E0103` `[25,27)` y `NX-E0102/not_indexable` `[90,94)`. Fallan también `planned_projection` en sus dos formas, `incompatible_prefix`, R2, X_nest_first, B_qr, B_rq, P1, c3 y w1. La causa es que la sombra separa la lectura de su binding, pero su enlace tardío solo copia el tipo concreto: pierde restricciones compatibles que la lectura debía aportar a la misma variable monomórfica (§4.1.1, §4.1.2.2–3). R2 queda **FALLA** antes de barrido. Los corpus c1-nueva/c2-nueva siguen reservados para validación; no se usaron para ajustar R1 ni R2.

La corrección de T5 se repitió con el binario release SHA-256 `821f77da1534…` (`r1-t5-correccion-sondas.json`, commit `0e6b8b8`). R1 con obligación activa pasó **45/45** filas de la matriz, pero **13/16** sondas: `planned`, `planned_full` e `incompatible_prefix` fallan. Su historial con T5 dio **40/45** y **3/16**. La corrección conserva las C independientes de O08 sin resolver la restauración ni la frontera de lectura.

R3 sustituyó la copia unidireccional final por una transacción de enlace entre sombra y binding, con snapshot completo de `InferenceGraph` y rollback si el enlace falla o queda pendiente. Se compiló bajo `heavy-lock` y se midió con el mismo binario y watchdog (`r3-sondas.json`, commit `0e6b8b8`): sin T5 **10/16** sondas y **37/45** filas; con T5 **10/16** y **38/45**. Fallan W0 de 78 B, `projection_nested_c1_1976`, 97 B, ambas `planned_projection` e `incompatible_prefix`; en la matriz fallan X_nest_first, W0, B_qr, B_rq, P1, c3 y w1. W0 obtiene `NX-E0102/not_indexable` `[52,53)` frente a `NX-E1303` esperada; 97 B conserva `NX-E0103` extra `[25,27)` y `not_indexable` `[90,94)`; `planned` produce cuatro `NX-E0102` en `[52,53)`, `[59,63)`, `[76,77)` y `[90,91)`. La causa está en la secuencia comprobada de `function()`: el enlace se ejecuta después de `block()`, cuando ya se evaluaron la proyección, las uniones y sus diagnósticos; si el enlace queda incompatible o pendiente se restaura. No reproduce las igualdades compatibles en orden de ocurrencia. **R3 FALLA la puerta dirigida; no se ejecuta barrido completo ni se declara parada del espacio de representaciones.** Ninguno de los corpus c1-nueva/c2-nueva se usó para ajustar R3. El siguiente diseño R4 está especificado en el documento de diseño; falta prototipo.

## Barrido completo de `txmarksproj` (release, `check`)

Los trece conjuntos/perfiles siguientes quedaron medidos con el binario `txmarksproj` SHA-256 `57f9db1da0d2…` frente a `main` `44bef37452e4…`, corpus congelados comprobados por SHA, cuatro hilos y un solo lote bajo `heavy-lock` cada vez. A/B/C/D de núcleo solo cuenta filas modeladas: A = `main` correcto y variante incorrecta, B = variante correcta y `main` incorrecto, C = ambos incorrectos iguales, D = ambos incorrectos distintos. `A full` incluye tipos y relación de procedencia. `Δ sin oráculo` es diferencia frente a `main` **sin calificar**. Los JSON íntegros están en `docs/validation/i07c-rediseno/barrido/` de la rama desechable; ningún lote informó error o hijo inconcluso.

| corpus/perfil | filas modeladas / total | core A/B/C/D | A full | Δ sin oráculo |
|---|---:|---:|---:|---:|
| c1 | 24.000 / 24.000 | 9/2762/5/2 | 5 | 0 |
| c2 · `f0-0002-b-v2` | 15.809 / 16.000 | 3/641/7685/72 | 1 | 13 |
| c2 · `f0-0003-b-v1` | 15.809 / 16.000 | 5/1091/2370/146 | 3 | 13 |
| corpus-b | 3.959 / 4.000 | 1/546/309/3 | 1 | 0 |
| perm | 3.822 / 4.044 | 4/340/4/0 | 9 | 0 |
| corpus-t | 0 / 10.584 | — | — | 4.781 |
| B 7021993 | 1.098 / 4.500 | 1/206/0/1 | 0 | 546 |
| B 4417061 | 1.138 / 4.500 | 1/221/0/3 | 0 | 546 |
| B 9908255 | 1.108 / 4.500 | 0/177/0/2 | 1 | 539 |
| dirigido5 | 0 / 6.000 | — | — | 123 |
| c1-nueva | 8.000 / 8.000 | 2/819/0/3 | 1 | 0 |
| c2-nueva · `f0-0002-b-v2` | 7.910 / 8.000 | 2/293/3938/34 | 0 | 7 |
| c2-nueva · `f0-0003-b-v1` | 7.910 / 8.000 | 3/555/1186/61 | 2 | 7 |

Los A del corpus nuevo hacen aplicable la parada del brief §5 por causa estructural común. Los detalles por familia y las adjudicaciones `takeempty` siguen abajo. Este barrido no es una prueba de aceptación `run`, debug/release ni PASS.

### Siete variantes adicionales de marca y concreción (binario release `09a4208a95d2…`)

Tras el barrido completo, se compiló bajo `heavy-lock` el prototipo con los interruptores desechables de alcance y concreción. Las siete variantes se midieron con las mismas 16 sondas y 45 filas T5/N; las salidas íntegras por programa y SHA están en `sondas-variantes-marca.json` de `i07c-rediseno-proto`. Ninguna supera ambas puertas dirigidas. Las ocho variantes anteriores ya tenían barrido completo de c1 y análisis de causa en la tabla de c1; estas siete añaden contraste del mecanismo, no sustituyen aquel barrido.

| flags además de `txmarks` sobre candidato 5 | sondas / 16 | matriz / 45 | falsación causal |
|---|---:|---:|---|
| `txmarksproj,txmarksprojstatement` | 14 | 44 | La proyección ajena deja de activar la marca; siguen A en c1-1976 y la unión de 97 bytes. |
| `txmarksproj,txmarksprojbefore` | 14 | 44 | Mismo resultado: posición anterior no identifica la dependencia de la obligación. |
| `txmarksproj,txmarksallowconcrete` | 14 | 44 | Permitir completar constructores reabre `NX-E0102` `[100,101)` y `[105,106)` en la proyección ajena; falla también la unión de 97 bytes. |
| `txmarksstatement,txmarksallowconcrete` | 15 | 42 | Recupera W0, pero `incompatible_prefix` pierde su rechazo de cuatro diagnósticos; B_qr y w1 vuelven a fallar. |
| `txmarksoccurrence,txmarksallowconcrete` | 15 | 42 | Igual pérdida de `incompatible_prefix`, B_qr y w1: ampliar el alcance no separa las dos vistas de evidencia. |
| `txmarksproj,txmarksprojstatement,txmarksallowconcrete` | 15 | 44 | Sigue el A de 97 bytes: añade `NX-E0103` `[25,27)` junto al único esperado `[16,18)`. |
| `txmarksproj,txmarksprojbefore,txmarksallowconcrete` | 15 | 44 | Mismo A de 97 bytes; anterioridad temporal no basta para asociar marca y constructor. |

Las siete fallan además O08, cuya derivación T5/N retira dos diagnósticos contaminados y todavía requiere validación independiente. La causa estructural común permanece: `deferred_join` representa simultáneamente disponibilidad de contexto previo y posibilidad de completar un constructor posterior. Filtrar la marca por función, sentencia, posición o forma del operando cambia ambas vistas a la vez; permitir la concreción global cambia las dos en dirección contraria. Ninguna variante de esta familia realiza el compromiso por ocurrencia con dependencias y dos vistas del prefijo que exige la regla candidata.

La medición de pasadas del binario congelado `txmarksproj` sobre los 24.000 programas de c1, con `NEXO_XV=…,txmarks,txmarksproj,passes`, cuatro hijos y watchdog de 6 GB/20 min, dio máximo **3**: 77 programas con una pasada, 23.882 con dos y 41 con tres; cero inconclusos. `pasadas-c1.json` guarda el SHA del binario y el histograma. No se observó la parada de cinco pasadas en este corpus; la cota general sigue sin demostrarse.

La sonda de pila de `scripts/f5-stack65.py` se **repitió válidamente bajo `heavy-lock`** con rustc 1.99.1 y las flags de `txmarksproj`: el control de 16 KiB desbordó como se esperaba (`0xc00000fd`), y `option_match126` y `result_match126` pasaron **5/5 cada uno a 65 KiB**. El test executable fue SHA-256 `239b807fc8f3…`; `pila65-txmarksproj.json` registra hashes, flags y salidas. Esta observación reemplaza la ejecución accidental anterior para esas diez sondas. **NO EVALUADO** aún: reserva exacta de `Checker::drive` y `resume_non_match` por el método exe/MAP de `f5-frame-sizes.py`; no se atribuye el margen ≥1.000 B sin esa medición.

### Dictamen de parada de esta continuación

**FALLA, no PASS.** El brief §5 ordena parar un diseño si un corpus nuevo presenta A de núcleo con causa estructural común. c1-nueva (2 A) y c2-nueva (2 A en `f0-0002-b-v2`, 3 en `f0-0003-b-v1`) se congelaron por SHA antes de medir `txmarksproj`; sus repros 4223, 7417, 5312, 6526 y 7479 identifican la misma contaminación entre marca de unión pendiente, plan de lectura/fijación y constructor posterior, incluso cuando la continuación es un `match`. Los mínimos de 97 y 115 bytes y c1-1976 permiten reproducir la causa fuera del generador. Las ocho variantes previas tienen barrido completo de c1; seis dan A=0 allí pero pierden W0/`incompatible_prefix` y quedan en 39/45, y las otras dos dan A=9. Las siete variantes adicionales medidas con el binario nuevo tampoco superan los mínimos y la matriz. No hay evidencia de una realización de la regla única de compromiso por ocurrencia; las flags de prototipo no se trasladan a producción.

**Puertas pendientes, sin veredicto favorable:** aceptación de todos los corpus en `check`/`run`, tres perfiles y debug/release; las 52 filas declaradas de `match` por cláusula (aunque las 51 de conservación dirigidas permanecieron iguales); 152 RD y corpus objetivo; 3.674, twin y fuzz; tamaño de marcos exe/MAP, coste de tiempo y cota general de pasadas; disposición implementada de los 17 tests y cada fila de defectos abiertos. `corpus-t`, dirigido5 y las formas no modeladas de las semillas B tienen diferencias frente a `main` aún sin esperado; se preservan como trabajo abierto, no como deuda cerrada. Las adjudicaciones `takeempty` de c2 y c2-nueva necesitan revisión independiente de B. El resultado es **reabrir arquitectura por Flujo A**, separando explícitamente evidencia de contexto al entrar en un nodo y evidencia de completación posterior, vinculadas a la misma ocurrencia con dependencias; ningún plan de Flujo B se escribe ni se integra código.

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

### Primer lote publicado del barrido masivo

El barrido reproducible de `scripts/i07c_rd_campaign.py` publicó c1 índices 0–1.999 en `i07c-rediseno-proto`, con binario base `44bef37452e4…` y prototipo `57f9db1da0d2…`, cuatro hilos, watchdog de 6 GB/20 min por hijo y cero resultados de arnés inconclusos. El resumen A/B/C/D íntegro está en `docs/validation/i07c-rediseno/barrido/summary-c1.json` de esa rama. `txmarksproj` y `txmarksright` tienen **A=1, B=199, C=0, D=0** en core; las otras seis variantes tienen **A=0, B=199, C=0, D=0** en este lote. Es solo 2.000/24.000 de c1, no una cifra global.

El índice 1976 explica A=1 de `txmarksproj` y `txmarksright`: `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=[(if p then (if p then r[0] else q) else (if p then [q,r] else r))];q=[ar];r=q;}`. `main` y el oráculo coinciden en `NX-E0103` `[85,87)`, `NX-E0102` `[150,151)`, `NX-E1303` `[158,159)` y `NX-E0102` `[172,173)`. Ambas variantes dan `NX-E0103` `[76,78)` y `[85,87)`: pierden la fijación y los tres diagnósticos posteriores. `txmarksconcrete` conserva los cuatro diagnósticos del `main` en esa fila, pero su sonda `planned_projection` sigue fallando. La traza `NEXO_XV=…,dbg` del mismo binario muestra el primer cambio en planning: candidato 5 y `txmarksconcrete` tienen `concrete_plan=[65]`, `read_plan=[]`; `txmarksproj` mantiene `concrete_plan=[65]` pero añade `read_plan=[65,74]` (sitios de `q` y `r`). La marca reconstruida expone como contexto de lectura una obligación que aún no puede decidir la fijación, y el cierre pierde las igualdades posteriores.

Los repros se añadieron a `scripts/i07c_rd_cheap.py` del prototipo y se repitieron en las ocho variantes con el mismo binario (16 sondas y matriz T5/N de 45). `txmarksproj`: 13/16 y 44/45, falla los tres repros de proyección; `txmarksright`: 12/16 y 39/45, falla los dos de proyección en unión además de W0 e `incompatible_prefix`; las otras seis: 14/16 y 39/45, pasan los tres nuevos pero pierden W0 y `incompatible_prefix`. Esta tabla distingue regresiones contra aciertos de `main` de pérdidas de arreglos de defectos que `main` ya tenía. Ninguna variante reúne todavía las sondas y la matriz requeridas.

El lote c1 0–7.999, publicado en `i07c-rediseno-proto`, da A core `txmarksproj=4`, `txmarksright=3` y 0 en las otras seis variantes; B core está entre 857 y 860. C y D son 0–2 y 0–1 respectivamente según variante; la tabla completa de core/full está en el resumen versionado de la rama de prototipo. Los cuatro índices de A de `txmarksproj` son 1976, 2159, 4798 y 6749. Los tres últimos comparten proyección `r[0]` dentro de una unión anidada: la variante añade `NX-E0103` de `r` o pierde la denuncia de la unión. El índice 6749 se redujo a 97 bytes: `fn Main(){var q=[];var r=[];let a=(if true then r[0] else (if true then q else r));r=q;r=[true];}`. `main`, candidato 5 y el oráculo T5/N dan solo `NX-E0103` `[16,18)`; `txmarksproj` y `txmarksright` añaden `NX-E0103` `[25,27)`. El fichero exacto sin LF final está en `docs/validation/i07c-rediseno/txmarksproj-proyeccion-union.n`. La variante conserva una marca de unión pendiente que hace perder la fijación independiente de `r`; el punto donde la obligación se compromete aún requiere traza específica de este mínimo.

En ambos mínimos de 115 y 97 bytes repetí `check` y `run` en release para `f0-0002-b-v2`, `f0-0003-b-v1` y `f0-0005-b-v1`: las doce comparaciones rechazan en ambos binarios con exit 1, pero la regresión diagnóstica indicada persiste idéntica en los tres perfiles y los dos comandos. Es una comprobación dirigida de aceptación; la campaña completa y debug/release siguen pendientes.

### c1 completo (24.000 programas; release, `check`, `f0-0002-b-v2`)

El resumen acumulado publicado en `i07c-rediseno-proto` llegó a 24.000/24.000 sin error ni resultado inconcluso del arnés. Cada celda es A/B/C/D; A es regresión frente a un acierto del oráculo, B arreglo, C igualdad de dos salidas erróneas y D dos salidas erróneas distintas. `OK` (ambos aciertan) se registra aparte en el JSON.

| variante | core A/B/C/D | full A/B/C/D |
|---|---:|---:|
| `txmarksproj` | 9/2762/5/2 | 5/1461/12711/1534 |
| `txmarksright` | 9/2761/4/4 | 4/1438/12726/1542 |
| `txmarksconcrete` | 0/2766/1/2 | 5/1459/12709/1538 |
| `txmarksstatement` | 0/2769/0/0 | 2/1435/12739/1532 |
| `txmarksoccurrence` | 0/2760/5/4 | 2/1429/12745/1532 |
| `txmarksstatement,txmarksright` | 0/2769/0/0 | 2/1446/12727/1533 |
| `txmarksoccurrence,txmarksright` | 0/2764/2/3 | 2/1442/12729/1535 |
| `txmarksoccurrence,txunify` | 0/2760/5/4 | 2/1429/12745/1532 |

Los nueve índices de A de `txmarksproj` son 1976, 2159, 4798, 6749, 12397, 13590, 16314, 22342 y 22400. `txmarksright` comparte ocho y sustituye 2159 por 13601. La familia común implica proyecciones de binding pendiente en una unión: reconstruir su marca cambia `read_plan` y puede añadir `NX-E0103`, perder `NX-E1303` o trasladar la incompatibilidad a una asignación posterior. Los mínimos de 97 y 115 bytes separan respectivamente una proyección dentro de la unión y una ajena a ella; no se deduce de esta tabla que las seis variantes con A=0 cumplan el objetivo, porque todas fallan los arreglos dirigidos W0 e `incompatible_prefix` y la matriz T5/N.

### c2 completo, primer perfil (16.000 programas)

En `f0-0002-b-v2`, `txmarksproj` da core A/B/C/D = **3/641/7685/72** y full A/B/C/D = **1/359/10804/392**; 7.408 core y 4.253 full son aciertos comunes. Hay 191 programas sin oráculo: 178 coinciden con `main` y 13 difieren; esas 13 diferencias no se clasifican como A o B. Los tres A de núcleo (índices 2739, 4319, 13512) contienen la misma familia de proyección de un binding pendiente dentro de una unión: en 2739 (función `Helper`) el `NX-E1303` pasa de `r[0]` `[126,130)` a `r` `[154,155)` y aparece un `NX-E0102` posterior; en 4319 y 13512 se añade `NX-E0103` de `r` `[76,78)`. El resumen por SHA está publicado en la rama desechable.

En `f0-0003-b-v1`, las mismas 16.000 fuentes dan core A/B/C/D = **5/1091/2370/146** y full A/B/C/D = **3/633/7922/676**; 12.197 core y 6.575 full son aciertos comunes. Los cinco A core son 1603, 2739, 3712, 4319 y 13512. Los dos nuevos en este perfil también reconstruyen prematuramente la marca de una unión anidada: 1603 desplaza `NX-E1303` `[146,150)` a `[156,157)` y añade `NX-E0102` `[163,164)`; 3712 sustituye `NX-E1303` `[183,187)` por `NX-E0102` `[205,206)`. Hay 191 filas sin oráculo, de las cuales 13 cambian. Son los mismos 13 índices `takeempty` del primer perfil. La reducción por llamadas independientes, el oráculo T5/N, la restitución de spans y la comparación de los campos completos y diagnósticos de llamada se repitieron para este perfil con 13/13 coincidencias (`adjudicacion-takeempty-f0-0003-b-v1.json`, rama desechable). B aún debe validar la derivación; no es PASS. El barrido de los demás corpus continúa bajo cerrojo en lotes publicados.

Los 13 cambios sin oráculo de c2 son exclusivamente de la forma `takeempty`. Quedaron **adjudicados como salida admisible del prototipo** por la derivación reproducible `scripts/i07c_rd_takeempty_adjudication.py` y su tabla por fila `docs/validation/i07c-rediseno/adjudicacion-takeempty.json` en la rama desechable. En cada programa, `q` ya tiene su asignación antes de `Take([]);Take(q);`: `Take([])` recibe el tipo concreto de la firma, y la igualdad de `Take(q)` es independiente; si falla, su rechazo atómico no cambia `q` ni la unión posterior (§4.1.2.1–2). Se retiran las dos llamadas solo para derivar con el oráculo T5/N las obligaciones de la unión, se restituyen sus offsets por bytes y se cotejan aparte los diagnósticos de las llamadas (idénticos en `main` y prototipo). Las 13 salidas del prototipo conservan **todos los C, tipos y relaciones** de la derivación y las llamadas comunes; ninguno queda sin programa o sin esperado. La validación independiente de esta reducción corresponde a B y todavía no constituye PASS.

| índice | diferencia de `main` frente a la derivación; el prototipo coincide |
|---:|---|
| 3254 | `NX-E0103` extra `[126,128)`; la unión conserva `NX-E1303` `[218,222)`. |
| 4551 | `NX-E0103` extra `[126,128)`; la unión conserva dos `NX-E1303`. |
| 4836 | `NX-E1303` extra `[195,197)`; quedan dos `NX-E0103` causales. |
| 6747 | `NX-E1303` extra `[255,259)`; queda el de `[229,230)`. |
| 7822 | `NX-E0103` extra `[128,130)` y relación de unión distinta; el origen derivado es `[173,174)`. |
| 8711 | `NX-E1303` extra `[194,195)` y `[212,213)`; queda el de `[171,176)`. |
| 8853 | `NX-E1303` extra `[217,219)`; queda el de `[199,200)`. |
| 10674 | `NX-E0103` extra `[181,182)`; queda el constructor `[126,128)`. |
| 11188 | `NX-E1303` extra `[257,261)`; quedan los tres sitios causales previos. |
| 13127 | `NX-E1303` extra `[217,218)`; queda el de `[202,203)`. |
| 13557 | Solo cambia la relación `expected_type`: origen derivado `[152,155)`, no `[172,175)`. |
| 14009 | `NX-E1303` extra `[228,229)`; queda el de `[205,210)`. |
| 15312 | `NX-E0103` extra `[126,128)`; queda `NX-E1303` `[193,195)`. |

### corpus-b completo (4.000 programas; release, `check`)

`txmarksproj` da core A/B/C/D = **1/546/309/3** y full A/B/C/D = **1/307/2218/280**; 3.100 core y 1.153 full son aciertos comunes. Las 41 fuentes sin oráculo coinciden con `main`. El único A es el índice 3799: `main` y oráculo denuncian los constructores `[76,78)` y `[85,87)`; `txmarksproj` añade `NX-E0103` `[67,69)`. La fuente contiene `q[0]` dentro de una unión anidada y otra unión con `[s,q]`; la marca de proyección reconstruida cambia el cierre de un constructor anterior. Es la misma familia estructural observada en c1 y c2. El resumen íntegro por SHA se publicó en `i07c-rediseno-proto` (`summary-b.json`), sin error de arnés.

### perm completo (4.044 programas; release, `check`)

`txmarksproj` da core A/B/C/D = **4/340/4/0** y full A/B/C/D = **9/261/1173/125**; 3.474 core y 2.254 full son aciertos comunes. Las 222 fuentes sin oráculo coinciden con `main`. Los A 3084, 3087 y 3088 son tres órdenes de las mismas asignaciones alrededor de una unión anidada con `r[0]`; el 3955 también contiene `r[0]` y una lectura de `q` tras `r=q`. En 3084, la variante traslada `NX-E1303` de `[143,147)` a `[153,154)` y añade `NX-E0102` `[165,167)`: reconstruir la marca deja que la asignación posterior altere el contraste anterior. El resumen `summary-perm.json` está publicado sin resultados inconclusos.

### corpus-t completo (10.584 programas; release, `check`)

El corpus-t **no tiene oráculo propio**: A/B/C/D no se pueden calcular, y los ceros del JSON no significan ausencia de regresiones. `txmarksproj` coincide con `main` en 5.803 salidas y difiere en 4.781. El índice histórico 3584 tiene esperado del ADR y demuestra al menos una regresión de la variante `txmarks` inicial; la variante `txmarksproj` conserva ese mínimo en la sonda barata. Las otras diferencias no se declaran arreglos ni regresiones sin adjudicación; el tramo no está cerrado. El resumen `summary-t.json` conserva los conteos y SHA, sin error de arnés.

### Semilla B 7021993 completa (4.500 programas; release, `check`)

Sobre los 1.098 modelados, `txmarksproj` da core A/B/C/D = **1/206/0/1** y full A/B/C/D = **0/110/531/117**; 890 core y 340 full son aciertos comunes. El A 2755 contiene `r[0]` en ambas uniones interiores: `main` y oráculo coinciden en `NX-E0103` `[85,87)`, `NX-E1303` `[160,161)` y `NX-E0102` `[167,169)`; la variante añade `NX-E0103` `[76,78)` y `[128,131)`, traslada `NX-E1303` a `[127,132)` y pierde el `NX-E0102`. Es el mismo desvío de marca y fijación por ocurrencia. Los 3.402 sin oráculo incluyen 546 cambios frente a `main`; siguen sin calificar. `summary-s7021993.json` quedó publicado sin error de arnés.

### Semilla B 4417061 completa (4.500 programas; release, `check`)

Sobre los 1.138 modelados, `txmarksproj` da core A/B/C/D = **1/221/0/3** y full A/B/C/D = **0/125/557/114**; 913 core y 342 full son aciertos comunes. El A 1646 tiene `q[0]` en un `value` anidado posterior: `main` y oráculo dan solo `NX-E0103` `[67,69)` y `[85,87)`; la variante añade `NX-E0103` `[76,78)` y dos `NX-E1303` `[137,142)` y `[172,177)`. La marca activada por la proyección altera otras igualdades de la sentencia. Los 3.362 sin oráculo incluyen 546 cambios sin adjudicar. `summary-s4417061.json` quedó publicado sin error de arnés.

### Semilla B 9908255 completa (4.500 programas; release, `check`)

Sobre los 1.108 modelados, `txmarksproj` da core A/B/C/D = **0/177/0/2** y full A/B/C/D = **1/98/559/93**; 929 core y 357 full son aciertos comunes. El A full 2065 conserva los códigos y primarios, pero cambia la relación `expected_type` del `NX-E0102` `[173,174)` de `[163,169)` a `[117,118)`. La fuente no contiene proyección; es una pérdida de procedencia de la base candidata, no una activación de `txmarksproj`, y sigue pendiente por S5a.5 y la procedencia A del brief §§1 y 4. De los 3.392 sin oráculo, 539 cambian sin adjudicar. `summary-s9908255.json` quedó publicado sin error de arnés.

### dirigido5 completo (6.000 programas; release, `check`)

El generador de casos con inicializador divergente, `return`, `break` y `while` no tiene oráculo por fila: A/B/C/D no son clasificables. Hay 5.877 salidas idénticas a `main` y 123 distintas. El resultado se publicó en `summary-dirigido5.json` con SHA y sin errores de arnés. Las 123 diferencias requieren esperado individual antes de calificar el tramo; no se cuentan como arreglos.

### c1-nueva completa (8.000 programas congelados antes de `txmarksproj`; release, `check`)

`txmarksproj` da core A/B/C/D = **2/819/0/3** y full A/B/C/D = **1/434/4499/462**; 7.176 core y 2.604 full son aciertos comunes, sin filas sin oráculo. Los A 4223 y 7417 son programas nuevos, no usados para ajustar la variante. En 4223 una unión anidada con `r[0]` pierde `NX-E1303` `[138,142)` y gana `NX-E0102` `[160,161)` tras `r=q`. En 7417, `r[0]` en el primer brazo y `[q,r]` en el otro hacen que la variante añada `NX-E0103` `[67,69)` y `[76,78)` a la única denuncia esperada `[85,87)`. Ambos confirman la causa estructural de marca y fijación de una ocurrencia pendiente. `summary-c1-nueva.json` quedó publicado con los SHA y sin error de arnés.

### c2-nueva, primer perfil completo (8.000 programas congelados; release, `check`)

En `f0-0002-b-v2`, `txmarksproj` da core A/B/C/D = **2/293/3938/34** y full A/B/C/D = **0/172/5439/171**; 3.643 core y 2.128 full son aciertos comunes. Los A 5312 y 6526 muestran de nuevo proyección `r[0]` en una unión anidada: en 5312 se pierde `NX-E1303` `[159,163)` y aparece `NX-E0102` `[183,184)`; en 6526 se pierde `NX-E1303` `[177,181)` y aparece `NX-E0102` `[191,192)`. Hay 90 filas sin oráculo: 83 coinciden con `main` y siete `takeempty` difieren (33, 2797, 3174, 4069, 5560, 7652, 7829). La reducción de `Take([]);Take(q);` conserva los diagnósticos de llamada en las siete y coincide en código, primario y campos de unión en seis. En 5560 el oráculo reducido da relación `expected_type` `[133,135)` para `NX-E0102` `[212,215)`, pero ese sitio es la lectura `ar` en `q=ar`: la cláusula (5) de «Origen de la relación `expected_type`» en `F0-0002-lagunas.md` obliga a seguir el alias hasta su inicializador concreto `[1]` `[71,74)`. Esa relación coincide en `main` y prototipo. Así quedan las siete salidas adjudicadas por la derivación más la regla normativa de alias, **pendientes de revisión independiente por B**. `adjudicacion-takeempty-c2-nueva.json` conserva ambas relaciones y la cita. `summary-c2-nueva.json` se publicó sin error de arnés.

### c2-nueva, segundo perfil completo (8.000 programas congelados; release, `check`)

En `f0-0003-b-v1`, `txmarksproj` da core A/B/C/D = **3/555/1186/61** y full A/B/C/D = **2/307/3951/342**; 6.105 core y 3.308 full son aciertos comunes. Reaparecen los A 5312 y 6526, y se añade 7479: dentro de un `match` con unión anidada y proyección `r[0]`, la variante pierde el `NX-E1303` exterior `[222,224)` que `main` y el oráculo conservan. Esa fila muestra que la marca de una unión interna también contamina la continuación de `match` (ADR `F0-0003` §3.3), no solo asignaciones posteriores. Las mismas siete filas `takeempty` difieren entre binarios y las siete coinciden con la derivación más la regla normativa de alias; B aún debe validar ambos perfiles. Los artefactos `summary-c2-nueva@f0-0003-b-v1.json` y `adjudicacion-takeempty-c2-nueva-f0-0003-b-v1.json` se publicaron sin resultados inconclusos de arnés.

`scripts/i07c_rd_accept.py` (rama desechable) quedó preparado para comparar aceptación de `check`/`run` en los tres perfiles con cuatro hijos y watchdog propio de 6 GB/20 min; una muestra de dos programas de c1 y un positivo de control verificaron el formato. La muestra no fundamenta aceptación global. El arnés de firmas A/B/C/D registra diagnósticos de `check`; los resultados de ejecución e `internal_error` se medirán por separado. La consulta `python -B scripts/f5-stack65.py --help` ejecutó accidentalmente las sondas de pila porque el script no implementa `--help`; quedó registrada como incidente operativo y **no** se usa como evidencia de pila ni PASS, pues no pasó por el cerrojo compartido.

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

## Registro histórico de `txmarks` inicial (antes de `txmarksproj`)

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

El fallo dirigido falsó `txmarks` inicial. Los `NO EVALUADO` de esta tabla describen únicamente aquella variante descartada; los barridos medidos después para `txmarksproj` y otras siete variantes están arriba y en los resúmenes versionados de la rama desechable. Los ceros del candidato 5 y las 56/1.007 de variantes históricas son datos de INFORME-1..8, no resultados propios de `txmarks`.

## Puertas del brief §7

| pieza | estado |
|---|---|
| Regla única, ejemplos, citas | **Hipótesis entregada** en `I07c-rediseno-restauracion-atomica-diseno.md`; falta demostración. El negativo T5 se derivó de `F0-0002-lagunas.md` en `2dfcf22`. |
| Prototipo desechable y comandos | **Entregado y falsado** en `i07c-rediseno-proto`; `txmarks` corrige `planned_projection` pero regresa en t3584, y `txunify` no cambia la decisión. |
| Barrido A/B/C/D de todos los corpus; matriz 45; tablas de `match`; aceptación y debug/release | **En curso para `txmarksproj`**: c1 y c2 en dos perfiles completos, 9/3/5 regresiones A de core. Quedan los demás corpus, aceptación global y debug/release. No se importan como propios los ceros del candidato 5: ese candidato falla `planned_projection`. |
| 152 RD, objetivo, 3.674, twin, fuzz, pila, pasadas, tiempo | **NO EVALUADO aún para `txmarks`**. La compilación release de la sonda sí pasó; no equivale a puerta completa. |
| 17 tests de §3.6 y cada fila abierta de la lista de defectos | **NO EVALUADO para `txmarks`**; siguen nominalmente abiertos hasta un nuevo diseño y derivación individual. No se editó ningún test. |

El corpus base sigue limitado a una función `Main`, seis hojas y profundidad tres; el oráculo histórico no modela `match`, `while`, bloques `{}` ni `AssertEq`, y precede a T5. Ni el recuento de muestras ni un positivo dirigido prueban ausencia de regresiones. La exploración continúa y **no se escribe plan de implementación**. Quedan pendientes las lagunas y piezas S5a.2/S5a.3/S5a.5, `match`, los 17 tests y las 50 filas abiertas; no se reclasifican como deuda sin programa.

## Continuación 5: sondas de representación (resultado parcial)

Los resultados reproducibles están en `docs/validation/i07c-rediseno/r45-sondas-inicial.json` de la rama desechable y se atan al SHA-256 del binario. La matriz usa el oráculo T5/N; estas filas son **sondas**, no el barrido de los trece conjuntos/perfiles. Ninguna variante de esta tabla autoriza todavía un plan de integración.

| instrumento | sondas /16 | T5/N /45 | contraejemplo y causa inmediata |
|---|---:|---:|---|
| R1 obligación activa con T5 | 13 | 45 | `planned`, `planned_full`: la obligación se drena antes de derivar `read_plan`; `incompatible_prefix`: T5 se aplica antes de comparar las ramas internas. |
| R2 vista de lectura | 10 | 37 | W0 pierde la igualdad monomórfica: la sombra copia el tipo final sin publicar los efectos compatibles de la lectura. |
| R3 enlace tardío | 10 | 38 | W0 y 97 B: el enlace después de `block()` no reejecuta proyección, unión ni reserva diagnóstica. |
| R4, proyección por replay de eventos con T5 temprano | 3 | 40 | t3584 mínimo añade `NX-E0102` `[85,86)`; W0 añade `NX-E0103` `[27,29)`. Reconstruir compuertas desde eventos no reejecuta las igualdades. |
| R5, primer prefijo acotado y T5 tardío | 14 | 44 | 97 B añade `NX-E0103` `[25,27)`; O08 añade `NX-E1303` `[137,138)` y `NX-E0102` `[155,158)`. La proyección del brazo de una unión se confundió con una proyección externa; el resultado contaminado seguía ofreciendo su variable al ancla exterior. |

R4 aquí es un instrumento **incompleto** respecto del replay por ocurrencia diseñado: no cuenta como quinta representación demostrada. Las dos causas de R5 están siendo corregidas por dependencia de ancestro de `if` y por impedir que `Tainted` ofrezca una variable de ancla. El siguiente resultado se publica separado; no se reemplaza esta medición. No se ejecuta barrido completo sobre una variante que falla sondas.

La segunda instrumentación de R5 (`r45-sondas-segunda.json`, SHA de binario en el archivo) alcanzó **15/16 y 44/45**: 97 B ya coincide, pero `projection_nested_c1_1976` conserva un `NX-E0103` extra `[76,78)` y O08 conserva `NX-E1303` `[137,138)` y `NX-E0102` `[155,158)`. En la primera fuente, una proyección dentro de otra `if` todavía calificó como evidencia externa a una unión posterior; el límite correcto debe atender al nodo de entrada, no solo a la relación de ancestro entre dos `if`. En O08 el tipo `Tainted` se entrega al consumidor, pero `if_finish_join` aún lo contrasta como valor de una rama. La tercera instrumentación excluye toda proyección cuyo propietario sea una unión y retira el resultado contaminado de ambos operandos del contraste exterior. Ambas son hipótesis aún sin medición; no se declara PASS ni barrido.

La tercera medición (`r45-sondas-tercera.json`) sigue en **15/16 y 44/45**. La proyección anidada aún añade `NX-E0103` `[76,78)`; registrar solo `active_if_span` al producir la proyección no identifica con fiabilidad su contención sintáctica. O08 cambia, pero no se corrige: faltan los `NX-E0102` `[100,101)` y `[107,109)`, y aparecen `NX-E1303` `[144,147)` y `NX-E0102` `[155,158)`. Vaciar todo el contexto esperado tras una condición inválida elimina errores propios de las ramas. La siguiente hipótesis conserva `expected` y retira solo el ancla exterior; para la proyección registra todos los intervalos sintácticos `if` y exige que la evidencia esté fuera de ellos. Sigue sin barrido ni PASS.

La cuarta medición (`r45-sondas-cuarta.json`, binario SHA-256 `bf5cb6e764153006b06ef6dda55fd3d1654fcd566c94c873f54110ea2decfb00`) da **16/16 sondas y 45/45 T5/N** tanto para `r5prefix,r1t5,r5t5` como al añadir `txmarks,txmarksproj`. La evidencia de proyección externa se calcula desde intervalos sintácticos de `if` y el contexto esperado solo se retira cuando dependía de un ancla exterior que una condición inválida no puede ofrecer. O08 conserva sus cuatro diagnósticos internos, `incompatible_prefix` conserva su contraste de ramas y los dos `planned` dejan de retipar la lectura. El binario se fijó como `.scratch/rd-bin/r5.exe` (reconstruible por el SHA); **entra al barrido completo**, sin PASS todavía.

### R5: primeros lotes del barrido (todavía incompleto)

En c1 0..4.000, la tabla publicada `summary-r5-c1.json` da core **A/B/C/D = 0/425/0/0** y full **1/227/2311/230**. El único A full es el índice 2882; el diferencial núcleo coincide. Fuente exacta de 189 bytes:

```nexo
fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];let a0=(if p then (if p then (if p then r else r[0]) else (if p then [q] else r)) else [q])==aar;q=[[3]];r=q;}
```

Para `NX-E0102/type_mismatch` con primario `[186,187)`, `main` y el oráculo derivan `expected_type` `[148,151)` (el `[q]` anterior); R5 la traslada a `[178,183)` (la asignación posterior `q=[[3]]`). La causa inmediata es pérdida de procedencia de prefijo al completar una lectura anterior con evidencia posterior, familia S5a.5 y regla A de `F0-0002-lagunas.md`. Todavía no se ha reducido a mínimo ni se ha adjudicado si otras filas full comparten la causa. El binario permanece fijo y el resto del barrido sigue; **no es PASS**.
