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

En `f0-0003-b-v1`, las mismas 16.000 fuentes dan core A/B/C/D = **5/1091/2370/146** y full A/B/C/D = **3/633/7922/676**; 12.197 core y 6.575 full son aciertos comunes. Los cinco A core son 1603, 2739, 3712, 4319 y 13512. Los dos nuevos en este perfil también reconstruyen prematuramente la marca de una unión anidada: 1603 desplaza `NX-E1303` `[146,150)` a `[156,157)` y añade `NX-E0102` `[163,164)`; 3712 sustituye `NX-E1303` `[183,187)` por `NX-E0102` `[205,206)`. Hay 191 filas sin oráculo, de las cuales 13 cambian; su coincidencia por índice y causa con `takeempty` aún requiere comprobación independiente. El barrido de los demás corpus continúa bajo cerrojo en lotes publicados.

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
