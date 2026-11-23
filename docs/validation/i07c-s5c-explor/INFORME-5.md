# Exploracion S5c, ronda 5: verificacion de aceptacion, tabla extendida, reclasificacion y correccion de `skip`

Subagente X, rama local `i07c-s5c-explor`, sin push. Inicio 2026-11-23 02:11, fin 02:50 (reloj de la maquina). Prototipo desechable; parche `candidato-5.diff` (= candidato-4 + `skipv3`, vigente por defecto). `main` = `base-release.exe` de `C:\projects\Nexo-i07c\.scratch\cand-bin\` (y un `main` reconstruido de `origin/main`; coinciden en las 54 filas, en los 387 `run` y en todos los outcome medidos); candidato = `candidato-4.diff` reconstruido en copia desechable (`git archive HEAD`, destino propio, bajo heavy-lock).

## 0. Resumen y rectificaciones de INFORME-4

1. **Aceptacion de los 28 y los 157: 0 cambios.** En `f0-0003` y `f0-0005` los 28 (14 «anadidos», 14 «quitados») y los 157 son RECHAZADOS tanto por `main` como por el candidato. Ningun «anadido» convierte en rechazado un programa que `main` aceptaba. Sobre TODOS los `match`/`fn+match`/`matchexpr` de los corpus (6.361 programas por perfil; `main` acepta 387): 0 aceptados->rechazados, 0 rechazados->aceptados, y `run` identico (exit y salida) en los 387 aceptados en ambos perfiles (`run5.py`).
2. **INFORME-4 se equivoco en tres cosas, que se rectifican aqui:** (a) la tabla de conservacion de 54 filas NO da 0 diferencias con el candidato: dan distinto **3** filas, y la causa NO es RM-M3 (sec. 3); (b) la cita de l.353/D-REF para los 14 «quitados» no los funda (sec. 4); (c) hay **21** programas de los 157 (no solo los 14 «anadidos» del 2.o brazo) donde RM-M3 ofrece un testigo que `main` no ofrece y cambia diagnosticos fuera del 2.o brazo (sec. 2).
3. **`skip` cambiaba la aceptacion** (reproducido: 4 programas de 6.000 en `f0-0002`, 7 en `f0-0003`/`f0-0005` de la familia dirigida; los 2 de B incluidos). Se corrige con una variante medida, `skipv3` (sec. 5): 0 cambios de aceptacion en todos los corpus y en la familia dirigida, sin perder las ganancias.
4. De los tests que fallan con 2g' (19 en el log de B, no 20): 2 los arregla `skipv3`, 6 fijan comportamiento erroneo de `main` con respaldo del oraculo o de l.301/13.9, 2 son andamiaje, y 5 quedan en contra o inconclusos (sec. 6), incluido un defecto nuevo del candidato.

## 1. Verificacion de aceptacion y `run` (punto 1)

`verif5.py` y `run5.py`; `check` en `f0-0003-b-v1` y `f0-0005-b-v1`:

| conjunto | n | main rechaza / cand rechaza | main acepta | cambios de aceptacion |
|---|---|---|---|---|
| 28 segundo brazo (14 anadidos + 14 quitados) | 28 | 28 / 28 en ambos perfiles | 0 | **0** |
| 157 «propios del match» | 157 | 157 / 157 | 0 | **0** |
| todos los match/fn+match/matchexpr (c2 + 3 corpus de B) | 6.361 por perfil | — | 387 | **0** (ni acepta-lo-que-main-rechaza ni al reves) |

`run` (387 aceptados por main, ambos perfiles): exit y salida identicos entre `main` (los dos binarios) y candidato: 0 diferencias.

## 2. Adjudicacion de los 157 uno a uno (punto 3 de la tarea anterior)

`adjud5.py`: para cada programa se deriva con el oraculo `let a0=U;` (U = union del primer brazo, mismos decl/asignaciones), se trasladan los spans y se compara con `main` y candidato el conjunto de diagnosticos FUERA del 2.o brazo. Se adjudicaron **los 157** (la muestra pedida era >= 30): 

| veredicto | n | % |
|---|---|---|
| mejora (candidato = oraculo, main no) | 110 | 70 % |
| neutra (misma salida fuera del 2.o brazo) | 18 | 11 % |
| **anadido / regresion** (main = oraculo, candidato no) | **21** | 13 % |
| no adjudicable (ninguno coincide) | 8 | 5 % |

Los 21 «anadidos» (lista en `adjudicacion-157-completa.json`) son efecto de RM-M3: con `rmm3b/e/f` apagados el candidato da lo mismo que `main` en los cuatro que se comprobaron (c2 10405, 6523; s4417061 2128, 135). Ejemplo: Sigma+`let a0=match move o{Some(n)=>{value: (if p then (if p then r else r) else 2);},None=>{value: [q];}};` -> `main`: E0103 `[93,95)` y `[102,104)`; candidato: solo `[93,95)` (el brazo 1 se ofrece y `[q]` fija `r`). Todos estan rechazados por ambos: no cambia la aceptacion, cambia QUE diagnosticos salen. Se reconoce que la «diferencia declarada» de la tabla (28 filas) no los incluye; ver sec. 7.

## 3. Las 3 filas de las 54 que difieren: causa

Reproducido con el candidato y con **todos los interruptores apagados** (`NEXO_XV=''`): las 3 filas siguen distintas, es decir, vienen de la infraestructura ALT-2 del primer commit (registro de ancla + contraste, d2-1), no de RM-M3 ni de ninguna otra pieza (quitar cada pieza no cambia nada). B tenia razon: la tabla esta roja desde d2-1 (13 diferencias con todo apagado: las 7 R-C + estas 6).
- `rejilla1-anidada-else-denunciada` y `B-161-anidada-else-pendiente-denunciada` (`(if p then 1 else (if p then s else true))`, `s` pendiente): `main` emite ademas una E1303 sobre `true`; el candidato no. El oraculo sobre la union sola (`let a0=U;`) da solo E0103 de `q` y `s`: **respalda al candidato** (l.353: una sola E1303 por union, las demas sin E1303 propia; l.367: el conflicto pendiente de `s` cuenta como ya denunciado). Mejora a nivel de union.
- `rejilla1-anidada-else-denunciada2` (`(if p then ar else (if p then 1 else q))`): `main` emite E0103 sobre `q`; el candidato no. El oraculo (`let a0=U;`): E0103 solo de `s` + E1303 sobre `1` (la misma salida del candidato): **respalda al candidato** (la hoja `q` compatible con la referencia fija su elemento; l.301).
La premisa «la tabla debe dar 0 con 54 filas» **no puede cumplirse** con ALT-2: las tres filas son diferencias de UNION con respaldo del oraculo. Se declaran en la tabla extendida (grupo `diferencias_declaradas_de_las_54`, clase `mejora_con_cita`, las citas de union l.353/l.367/l.301, que aqui si aplican).

## 4. Reclasificacion de los 14 «quitados» (segundo brazo)

Las citas de l.353/D-REF **no fundan** estos quitados (regulan la union `if`, no el testigo que un brazo ofrece al siguiente; es la laguna (a) del `match` con valor): se retiran de la tabla. Medicion por fila (`quitados-reclasificados.json`; oraculo sobre el analogo `(if p then U else V)` restringido al 2.o brazo; ablacion por pieza):
- **Respalda al candidato el analogo: 7** (c2 7888, 13377; s4417061 996, 1096, 1190, 4363; s9908255 1143): el analogo tampoco emite la E1303 del 2.o brazo. Clase `quitado_analogo_respalda_candidato`; sin cita normativa; indicativo.
- **Respalda a `main` el analogo: 7** (s7021993 238, 561, 1253; s4417061 2805, 3282; s9908255 130, 1876): el analogo SI emite esa E1303; el candidato la pierde: **empeora** respecto del analogo. Clase `quitado_analogo_respalda_main`.
- Causa por pieza (quitar la pieza restituye `main` en esa fila): `retfail` 3, `rmm3e` 2 (+1 con `retfa`, +1 con `atomic0`), ninguna sola pieza en 7. **Ninguno de los 14 es de finalizacion (`e0103rule`)**: los quitados de `e0103rule` que reproduce B (R1, R2, R4: `NX-E0103` sobre `r[0]`/`q[0]`) son otra familia que no esta en estos 14 (estan entre los 157 y el oraculo los respalda: solo E0103 sobre constructores, l.301). Resultado honesto sobre el ADR: **0 mejoras con cita, 7 neutras con analogo a favor (no normativo), 7 que empeoran respecto del analogo**; el ADR no fija nada para los 14.

## 5. `skip` y la aceptacion (punto 3 de esta ronda)

Reproducido: `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}` -> `main` rechaza (check y run exit 1, NX-E0103 `[66,67)`); el candidato acepta y `run` da exit 0. Causa medida: `v` (inicializador divergente) es un nodo sin `array_origin`; la unica NX-E0103 de `main` cae sobre el NOMBRE `a` de `v=a` (el primario «en un nombre», el defecto que `skip` retira) pero es la UNICA razon de rechazo de una recursion `v = Array<v>` que el ADR rechaza (4.1.2.2, «Recursivo sin solucion finita»); al suprimirla no queda ningun otro diagnostico y el programa se acepta. La condicion «sin `array_origin`» es demasiado gruesa.

Variantes medidas (familia dirigida de 6.000 programas, `gen_dirigida5.py`, semilla 31415926, SHA-256 `684d6234...5dd5e09`, `var` con inicializador divergente/`return`/`break`/`while`, ciclos de arrays; 3 perfiles):

| variante | idea | cambios de aceptacion (0002 / 0003 / 0005) | programas con otro numero de diagnosticos que `main` |
|---|---|---|---|
| `skip` original (candidato-4) | omitir si no hay `array_origin` | **4 / 7 / 7** | 127 / 270 / 270 |
| `skipv1` | omitir solo si hay un constructor pendiente en el componente | no arregla los dos casos de B | — |
| `skipv2` | omitir solo en uniones (`join.branch`), emitir en asignaciones | 0 / 0 / 0 | 33 / 54 / 54 (**pierde ~210 ganancias**: ya no arregla p. ej. `[m1[7],m1]`) |
| **`skipv3`** (adoptada) | omitir, pero guardar el diagnostico y emitirlo al cerrar la comprobacion SOLO si el programa no tiene ningun otro error | **0 / 0 / 0** | 123 / 264 / 264 (ganancias conservadas) |

`skipv3` en todos los corpus (`acept_corpora.py`): c1, c2 (`f0-0002`, `f0-0003`), `corpus-b`, `perm`, `corpus-t` y los 3 de B: **0 cambios de aceptacion en 88.128 programa-perfil** y **0 regresiones de core**; tablas A-D iguales a las de la ronda 4 (c1 B=2.760, etc.); matriz 45/45; las 82 filas de la tabla extendida sin cambio. Los dos programas de B quedan rechazados con NX-E0103 como `main`.

## 6. Tests que fallan con 2g' (19 en el log de B; `b-d2plan/tests.log`)

Aviso de metodo: el log de B es de **candidato-3** (2g'), no de candidato-4/5. Clasifico con el comportamiento por CLI de candidato-4 (los programas de cada test) y el oraculo donde modela; no recompile la suite de tests. `L` = literal del ADR.

| test | esperado (test) vs obtenido | veredicto |
|---|---|---|
| `i07c_causal_obligations::mixed_nested_join_keeps_constructor_and_shape_sites` | E0103 `[]` + E1303 `[2,3]` vs solo E1303 | **fija error de main**: el elemento de `[[]]` lo fija la continuacion compatible `[[false],[true]]` (l.301, 4.1.2.2); el oraculo da solo la E1303 |
| `…::adr_gap_nested_union_and_array_element_keep_current_causal_sites` | E0103 `[]` + E1303 `br` vs solo E1303 | fija error de main (idem; oraculo = candidato) |
| `…::failed_outer_condition_keeps_nested_join_constructor` | idem con indexacion | fija error de main (oraculo = candidato) |
| `…::pending_inner_constructor_preserves_outer_branch_comparison` | E1303 sobre `false` vs ausente | **cita en contra**: el test cita 4.1.2.5; el oraculo (l.353 + l.367: el conflicto pendiente cuenta como ya denunciado) respalda al candidato: lecturas en conflicto -> Flujo A |
| `…::unresolved_constructor_survives_failed_nested_union_condition` | E0103 `t0` + E1303 `[5]` vs sin E0103 y E1303 sobre `true` | **cita en contra** (N(d), 4.1.2.5: una condicion fallida no descarga el constructor); no modelado; causa `retfail` (testigo): **inconcluso/probable empeora** |
| `…::failed_outer_array_keeps_nested_element_cause` | 3 E0103 vs 2 | fija error de main: la E0103 de `[t2]` es consecuencial de un interior sin resolver (13.9, l.301); no modelado: probable mejora |
| `i07c_mutation_regressions::anchored_first_branch_keeps_independent_pending_join` | E0102 + E1303 `[63,139)` + `[134,137)` vs solo E0102 | **empeora**: el oraculo da E0102 + E1303 `[134,137)`; el candidato pierde una E1303 correcta (y `main` anade una de mas): **defecto nuevo del candidato**, no cubierto por los corpus (hoja `s<1`), a investigar |
| `…::linked_array_constructors_keep_each_causal_inference_site` | E0103 `[80,82)` + `[130,137)` vs solo la primera | fija error de main: el literal exterior `[k0,k0]` con elemento sin resolver es consecuencial (13.9, l.301) |
| `…::cyclic_constructor_errors_keep_inference_code_at_each_site` | E0103 en `m1` (nombre) vs sin ella | fija error de main: l.301, E0103 solo sobre constructor fuente (primario en un nombre) |
| `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` | 2 E0103 + 2 E0102 vs 3 E0102 + E1303 | **cita en contra** (4.1.2.2, 4.1.2.7); no modelado: probable empeora |
| `…::planned_projection_child_does_not_use_failed_branch_as_witness` | 4 E0102 + E1303 vs 6 E0102 | **cita en contra** (4.1.2.2-5, 4.7): cambia E1303 por E0102 y anade 2; probable empeora |
| `…::failed_nested_union_does_not_anchor_an_outer_pending_array` | E1303 `[[q]]` vs otra | **inconcluso**: con candidato-4 `main` y candidato dan lo mismo por CLI (el log es de candidato-3): hay que recompilar el test |
| `i07c_normative_survivors::independent_causal_sites_survive_deferred_resolution` | E0103 `[28,30)` + `[73,77)` vs solo la primera | fija error de main (literal exterior consecuencial, 13.9) |
| `…::deferred_projection_and_constructor_programs_finish_with_causal_results` | 3 E0103 vs 2 | fija error de main (primario `[34,36)` en un nombre; l.301) |
| `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` | auditor de lineas | **artefacto**: las lineas de `types.rs` se desplazan; no fija semantica |
| `check::types::quota_tests::deferred_if_join_charges_the_reduced_boundary` | cargas de cuota (`limit 8`: 0 vs 1) | **andamiaje**: el propio test dice que los umbrales reducidos «pertenecen a I10a»; sin cita normativa de cifra |
| `…::failed_branch_join_keeps_reduced_quota_error_prefix` | idem | andamiaje |
| `ir::i07c_tests::adr_gap_recursive_array_unification_reports_inference_error` | rechazo | **lo arregla `skipv3`** (el candidato aceptaba) |
| `ir::i07c_tests::deferred_join_has_one_diagnostic_per_source_site` | rechazo | lo arregla `skipv3` |

Recuento: 2 arregla `skipv3`; 8 fijan error de `main` con respaldo del oraculo o de l.301/13.9 (mixed_nested, adr_gap_nested, failed_outer_condition, failed_outer_array, linked_array, cyclic, independent_causal_sites, deferred_projection); 1 lecturas en conflicto (pending_inner); 1 empeora con oraculo (anchored_first_branch); 3 con cita en contra no modelada (unresolved_constructor_survives, incompatible_prefix, planned_projection); 1 inconcluso (failed_nested_union); 3 andamiaje/artefacto (2 quota + auditor). **Bloqueantes honestos: `anchored_first_branch...` (defecto del candidato frente al oraculo) y los 3 con cita en contra.**

## 7. Tabla de conservacion extendida (82 filas)

`match-conservacion-ext.py/.json/.sha256` (copia ampliada; el original de `i07c-estructural-r2` no se toca): 51 filas `conservacion` (candidato = `main`), 28 `diferencias_declaradas` («diferencia declarada, no normativo»; clase y fuente, perfil, salida de `main` y del candidato por bytes — codigo, subcausa, primario, relaciones y tipos de cada diagnostico — y outcome de ambos; mismo resultado en `f0-0003`/`f0-0005`) y 3 `diferencias_declaradas_de_las_54` (sec. 3). Clases de las 28: `anadido_rmm3f` 14, `quitado_analogo_respalda_candidato` 7, `quitado_analogo_respalda_main` 7 (sin la cita falsa de l.353/D-REF). SHA-256 del conjunto: `60bf5ab264920d445f8b4129a87ba299c338ffd73cbb0a196cfec47a0123cf90`. `--check CANDIDATO` falla si cambia CUALQUIERA de las 82 filas (la `--check --main MAIN` comprueba que `main` da las 51 y los valores de `main` de las 31 declaradas): control y las dos formas dan «ninguna». **6 mutaciones** (`match-conservacion-ext-mut.py`): fila de menos, fila de mas, outcome alterado, diagnostico anadido, perfil cambiado y fila declarada alterada -> las 6 con exit 1, el control con exit 0 (`match-conservacion-ext-mut.out`). Con `skipv3` las 82 filas no cambian.
Pendiente: los 21 «anadidos» de la sec. 2 no estan como filas declaradas (el brief fijaba 28); conviene anadirlos (lista en `adjudicacion-157-completa.json`).

## 8. Limites e incidencias

- Los 21 anadidos y los 8 no adjudicables no se resolvieron; el analogo `(if p then U else V)` no modela el `match` (laguna a).
- La clasificacion de tests usa candidato-4 por CLI, no la suite recompilada; el log es de candidato-3. No se ejecuto `validate.sh`, `nextest` ni `twin`.
- `skipv3` guarda el diagnostico omitido y lo emite si no hay otro error en ese punto de la comprobacion (puede emitirse antes de errores posteriores del mismo cierre); prototipo desechable con clones y `eprintln!`.
- Se retira de INFORME-4: «0 diferencias en la tabla de 54», «mejora con cita para los 14 quitados» y la idea de que RM-M3 solo toca el 2.o brazo.
- Cerrojo `heavy.lock` con espera; barridos de procesos cortos fuera del cerrojo. Binarios, destinos y copias en `.scratch/` se borran al terminar.
