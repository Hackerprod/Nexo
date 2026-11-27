# I07c R6 — guía C1 v13, referencia ejecutable

Estado: **v13 propuesta para B r19, sin PASS**. Por decisión D7 del coordinador, la base de C1 es el árbol `1e4457fec7c02dc426afa74b25ae4660521c73a8` de `i07c-rediseno-c1c2-ref` (padre C0 `96d564496b1072d09a4a169c3be801d69aa3af96`). La rama de referencia se devolvió después a C0 en `ac962629d6a9b8b21ad8e7c021e61adafc84dc85`; se recupera el árbol por el SHA del commit, nunca por la punta de la rama. El árbol dio gate cruzado completo 20/20, 151.188 filas, A/D/A-unmodeled 0/0/0 y owned C1 3/74, 0/45, 0/13, F06=`main`; `validate.sh` **falló** por pánico `type_nodes[&id]` tras rollback. Luna corrige ese pánico y repite ambas puertas antes de entregar C1. C2–C6 quedan a verificar con gate completo por commit, sin cifras de ejecución atribuidas.

## Diferencia declarada de `main` con dueño C4

Una igualdad recursiva rechazada no publica su propuesta ni cancela deberes de constructor ya publicados por igualdades compatibles anteriores. La cardinalidad no es «un E0103 por ciclo» ni «un E0103 por representante»: cada constructor anterior cuyo elemento sigue irresuelto conserva su propio C (§4.1.2.2), salvo que su obligación dependa realmente de un error causal previo (§§4.1.2.5 y 13.9). En el programa de 78 bytes, `[]` y `[v]` siguen sin elemento resuelto tras rechazar `v=[a]`; por eso el esperado ADR es `[62,64)` y `[67,70)`. El único E0103 `[73,76)` de `main` no puede derivarse de esta regla sin enmendar el ADR. `EqualityId` debe conservar las dos obligaciones previas, nunca absorberlas por compartir componente o ciclo.

El árbol base C1 es `1e4457fec7c02dc426afa74b25ae4660521c73a8`
(ejecutable release SHA-256 `2c4612ffaa8daa1f10a365ec1d6ca27ac15bdc5d884aa322730447e6cc1a9831`);
la punta publicada de la rama experimental es `ac962629d6a9b8b21ad8e7c021e61adafc84dc85`,
idéntica en código a C0. Los cuatro barridos y las dos puertas mecánicas de
la continuación 28 figuran en `docs/validation/i07c-rediseno/C1C2-adr-blocker-28.md`.

## Referencia C1 y resultado reproducible

Árbol `1e4457f`, padre C0 `96d5644`. Diff neto: `src/check/types.rs` (+301/−119) y la aserción E3 de `src/ir/i07c_tests.rs` (+1/−1). `EqualityId`, `EqualityRecord` mínimo, `NodeJournal` por nodo y `PendingDeferredJoin.equality` están **presentes**. `ReadContext`, `ResultStatus` y el modelo final de `ConstructorDuty` quedan para C2–C4. El SHA de blob de `types.rs` es `364585898ab62920e965e5743e40ca4088f9f2c4`; el de `i07c_tests.rs` es `b1b8a5e965b350318923aa00c9b4da29de30e4f5`.

`begin_equality` asigna `EqualityId`, guarda operandos, `found` y el inicio de obligaciones, y abre `NodeJournal`. Las escrituras de nodos tocados se restauran en orden inverso si `finish_equality` rechaza la igualdad; también se retiran las obligaciones creadas por ese intento. `recursive_path_origin` recorre las aristas del camino causal y distingue deberes de constructor previos con testigo de elemento. `recursive_equality_diagnostic` usa el origen anterior **solo si hay exactamente uno**, excluyendo el constructor de la sentencia actual; con cero o varios conserva `request.found`. El `PendingDeferredJoin` guarda `recursive_origin` separado de `found` y retiene el `EqualityId` activo. La regresión `deferred_join_has_one_diagnostic_per_source_site` exige `[104,107)`; la iteración anterior que incluía el deber actual produjo `[110,113)` y falló.

Con `main` de `31d91db` y el ejecutable release de `1e4457f` (SHA-256 `2c4612ffaa8daa1f10a365ec1d6ca27ac15bdc5d884aa322730447e6cc1a9831`), `owned_rows_gate.py --commit C1` dio **3/74, 0/45, 0/13**, F06=`main`, cero fallos. `candidate_gate.py` ejecutó 20/20 corpus y 151.188 filas: A/D/A-unmodeled/H nuevas 0/0/0/0; informe SHA-256 `c29d9c7a20cd129cf903b63c886fe10297954a0bb4569767d8b1c02fe07bdbe6`. El `validate.sh` de ese árbol falló por el índice de nodo borrado tras rollback. `atomic_78` y W0 siguen bajo C3; W0nounion es su control positivo.

## Decisiones del cierre desplazado a C2/C3

1. **`Err` y `Join` creados.** El `EqualityId` dueño cancela todo `Join` creado por el intento rechazado, incluidos los de hijas fusionadas. No cancela obligaciones independientes publicadas antes del intento. Invariante: ningún `Join` listo apunta a un `EqualityRecord::Rejected`.
2. **Aristas después de `Waiting`.** La reversión vacía el journal y conserva solo la obligación con operandos, dueños y ordinal. Su activación reejecuta el contraste sobre el grafo publicado en un journal nuevo; solo una igualdad compatible publica aristas, una vez. Nunca se publican escrituras antiguas después de restaurar.
3. **`dependencies`.** No existe aún en el `EqualityRecord` mínimo de C1. C2 calcula el conjunto de `EqualityId` publicados que alcanzan los operandos y lo usa para invalidar sufijos provisionales y reactivar `Waiting`; si C2 no lo usa efectivamente, no puede afirmar su cierre. `ConstructorDuty.reaching_equalities` se migra en C4 sin doble autoridad.
4. **E0103 propio.** Lo origina el ciclo que `solve(EqualityId)` detecta o el deber de constructor registrado con ese mismo ID que llega sin solución. Un deber previo de otro ID no se consume por compartir representante, span ni diagnóstico.
5. **Journal anidado.** Una hija compatible fusiona sus primeras escrituras y obligaciones creadas con el padre; una hija rechazada restaura solo las propias. El rechazo del padre restaura también las hijas fusionadas; `next_variable`, `FactId`, cuotas y trabajo permanecen monotónicos.

El `JoinRecord.first_incompatibility` de C3 reemplaza el mapa experimental `union_denounced`; registra el primer contraste incompatible por `ConsumerId` de la `if`. La unión sin segunda continuación no reserva E1303. En W0 la continuación `q` queda ligada a su ancla solo si el contraste publica; el rechazo conserva el E1303 propio y permite que la igualdad independiente `[r,q]` resuelva `r` después. En F06 C1/C2 la puerta exige una salida completa de `main` u oráculo; una salida parcial falla.

Pruebas dirigidas antes de puertas completas: los tres recursivos y `deferred_join_has_one_diagnostic_per_source_site`; `atomic_78`, W0 y W0nounion como controles cruzados de C3; F06 con firma completa; negativo de cuota E0003 y positivo sin unión. Ninguna prueba inventa un esperado desde el código bajo prueba.

## Inventario completo: fila propia → mecanismo mínimo → commit

Los IDs se transcriben de `r9-commit-gates.json` v9. `frozen13` repite IDs de `directed74`; se indica para que las tres listas queden cubiertas. Los grupos con intervalo son inclusivos y no omiten IDs.

| Familia y filas propias | Mecanismo mínimo exigido | Primer commit |
|---|---|---|
| dirigidas `recursive_69`, `recursive_78_equivalent`, `recursive_127_equivalent` | origen del constructor en el `Join` recursivo pendiente; sin journal nuevo | C1 |
| dirigida `atomic_78`; matriz `W0` | `JoinRecord` de la `if` y dependencias causales de su ancla; cierre del constructor consecuente | C3 |
| matriz `W0nounion` | control positivo gemelo de W0; conservar `checked` | C3 |
| dirigidas `B_M1`, `B_M2`, `planned`, `ST1_equivalent` | ampliar `EqualityRecord` y journal de C1 con contexto de entrada, origen por componente y completación | C2 |
| dirigidas y `frozen13` `RC1_read_context_variable`, `RC2_read_context_committed`, `RC3_read_context_no_retroactive` | `ReadContext` por entrada/epoch y ausencia de retroactividad | C2 |
| dirigidas `X_02`, `X_03_array_annotation_equiv`, `X_04_argument_equiv`, `X_05_return_equiv`, `X_06_operand_equiv`, `X_07_three_levels_equiv`, `X_08_outer_condition_equiv`, `SB_equivalent` | `ResultStatus` de `if`, distribución de esperado y préstamo dirigido | C3 |
| dirigidas `join_record`, `W2_no_retroactive_context` | `JoinRecord`, testigo parcial y contexto exterior estable | C3 |
| dirigidas y `frozen13` `W1_atomic_compound`, `W1a_without_later_assignment`, `M2_partial_witness_chain` | `JoinRecord` anidado, testigo y orden de reservas | C3 |
| dirigidas y `frozen13` `RS1_condition_contaminated`, `RS2_independent_child_errors`, `RS3_partial_in_array` | `ResultStatus` y errores independientes | C3 |
| matriz `R1`, `R1twin`, `R2`, `R3`, `F06`, `M2`, `w1` | `ReadContext` C2 más `ResultStatus`/`JoinRecord` C3 | C3 |
| dirigidas `T5_constructor_equivalent`, `S_A_independent_constructor_equiv`, `completion_unresolved` | deber de constructor y reserva C independiente | C4 |
| dirigidas y `frozen13` `CD1_independent_constructor`, `CD2_independent_error_and_duties` | `ConstructorDuty.reaching_equalities` y cierre de reservas | C4 |
| matriz `O01`, `O03`, `O07`, `O08` | deberes de constructor y punto fijo | C4 |
| dirigidas `completion_conflict`, `suspended_68`, `waiting_projection_valid`, `waiting_projection_candidate_conflict`, `waiting_projection_refuted` | activación privada, `Waiting`/`Suspended` y replay | C5 |
| dirigidas y `frozen13` `SU1_suspended_order`, `SU2_suspended_chain` | suspensión y cierre por ordinal | C5 |
| dirigidas `B3_01`–`B3_35` (todos los enteros 01–35) | cierre integral de igualdad, deberes, replay y cuota | C5 |
| matriz `R3c`, `R2b`, `R3b`, `R4`, `X_R1na`, `X_R1ord`, `X_flat3`, `Y_pend_if`, `Y_pend_if2`, `X_mix3`, `X_R4asg_r`, `X_R1inc`, `X_R1pre`, `X_R2na`, `X_R4na`, `X_nest_first`, `Y_conc_if`, `Y_pend_if_asg` | orden/activación de `Waiting` y `Suspended`, replay | C5 |
| matriz `A`, `A_reord`, `A_ctrl`, `A_noasg`, `B_qr`, `B_rq`, `M2_noasg`, `M2_ctrl`, `DREF_valid`, `P1`, `c3`, `P2`, `P3`, `w3` | cierre de representantes, proyección y cuota tras replay | C5 |
| ninguna fila nueva de 74/45/13; todas las anteriores acumuladas | retirar rutas viejas, cierre final, A núcleo/completo=0 | C6 |

Esta tabla es de **propiedad contractual**, no un PASS de ninguna fila del implementador. El árbol `1e4457f` fundamenta solo los resultados de corpus y filas propias descritos arriba; su puerta mecánica falló y Luna debe corregirla. C2–C6 requieren sus propios gates completos y validación independiente. La familia D7 de diferencias de `main` se conserva hasta C4 conforme a R6-final v13 §5; sus tres residuales de `dirigido5` y el literal de 78 B se adjudican allí sin filtro por índice o span.
