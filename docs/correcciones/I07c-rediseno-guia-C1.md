# I07c R6 — guía C1 v12, referencia ejecutable

Estado: **v12 falsada; v13 bloqueada por contradicción normativa**, sin PASS. Autoridad histórica: R6-final v12 §§3–6 y `r9-commit-gates.json` v8. La referencia `i07c-rediseno-c1c2-ref` `5a05dc88ccebd523c804e8695983a516b4edbc73` portó el journal y falló el programa de 78 bytes; evidencia en `docs/validation/i07c-rediseno/C1C2-reference-blocker-27.md`. La continuación 28 obtuvo un gate de corpus verde en una referencia experimental posterior, pero la derivación manual de F0-0002 §4.1.2.2 exige **dos** NX-E0103 previos en ese programa, mientras `main` y esa referencia emiten solo el NX-E0103 final. La prueba y la imposibilidad de satisfacer simultáneamente ambos esperados están en `docs/validation/i07c-rediseno/C1C2-adr-blocker-28.md`. **Ninguna de esas referencias acredita C1 o v13.** La rama de referencia fue devuelta a C0 por `ac962629d6a9b8b21ad8e7c021e61adafc84dc85` tras hallar un pánico en `validate.sh`. Los cortes posteriores quedan a verificar por Luna con el gate completo.

## Invariante del ciclo que bloquea v13

Una igualdad recursiva rechazada no publica su propuesta ni cancela deberes de constructor ya publicados por igualdades compatibles anteriores. La cardinalidad no es «un E0103 por ciclo» ni «un E0103 por representante»: cada constructor anterior cuyo elemento sigue irresuelto conserva su propio C (§4.1.2.2), salvo que su obligación dependa realmente de un error causal previo (§§4.1.2.5 y 13.9). En el programa de 78 bytes, `[]` y `[v]` siguen sin elemento resuelto tras rechazar `v=[a]`; por eso el esperado ADR es `[62,64)` y `[67,70)`. El único E0103 `[73,76)` de `main` no puede derivarse de esta regla sin enmendar el ADR. `EqualityId` debe conservar las dos obligaciones previas, nunca absorberlas por compartir componente o ciclo.

## Referencia C1 y resultado reproducible

Rama nueva `i07c-rediseno-c1-ref`, SHA `88205510cdfe8f9df3eef1236a33a0487e9474cf`, padre C0 `96d564496b1072d09a4a169c3be801d69aa3af96`. Diff: solo `src/check/types.rs` (+111/−13) y la aserción E3 previamente aprobada de `src/ir/i07c_tests.rs` (+1/−1). No contiene código R1–R5, `NEXO_XV`, `union_denounced`, `ReadContext`, `ResultStatus`, `ConstructorDuty` nuevo ni el WIP de Luna.

El solver ya detectaba el ciclo, pero `Frame::UnifyAfterUnion` convertía el E0103 en `PendingDeferredJoin` y guardaba `request.found` (asignación posterior) en lugar del origen del constructor causal. `Checker::recursive_path_origin` recorre las aristas reales de nodos desde los operandos, toma el constructor más temprano con testigo de elemento en el camino que cierra el ciclo y, si no hay tal testigo, conserva `request.found`. `recursive_equality_diagnostic` emite E0103 con ese origen; el `PendingDeferredJoin` lo guarda separado de `found` y lo usa en su cierre. Ninguna decisión mira nombre, texto fuente ni una posición fija. La regresión `deferred_join_has_one_diagnostic_per_source_site` conserva un solo E0103 y ahora exige `[104,107)`.

Con `main` compilado exactamente de `31d91db` y el ejecutable release de esta rama, `owned_rows_gate.py --commit C1` bajo el manifiesto v12 exige **3/74, 0/45, 0/13** y F06 completa de `main`; su salida debe tener cero fallos. `atomic_78` y W0 siguen con E0103 adicional en el árbol aislado; W0nounion sigue `checked`. En v12 los tres IDs son de C3. El prototipo con `atomic0` no es una referencia C1.

## Decisiones del cierre desplazado a C2/C3

1. **`Err` y `Join` creados.** El `EqualityId` dueño cancela todo `Join` creado por el intento rechazado, incluidos los de hijas fusionadas. No cancela obligaciones independientes publicadas antes del intento. Invariante: ningún `Join` listo apunta a un `EqualityRecord::Rejected`.
2. **Aristas después de `Waiting`.** La reversión vacía el journal y conserva solo la obligación con operandos, dueños y ordinal. Su activación reejecuta el contraste sobre el grafo publicado en un journal nuevo; solo una igualdad compatible publica aristas, una vez. Nunca se publican escrituras antiguas después de restaurar.
3. **`dependencies`.** No existe en C1 v12. C2 calcula el conjunto de `EqualityId` publicados que alcanzan los operandos y lo usa para invalidar sufijos provisionales y reactivar `Waiting`; si C2 no lo usa efectivamente, no puede afirmar su cierre. `ConstructorDuty.reaching_equalities` se migra en C4 sin doble autoridad.
4. **E0103 propio.** Lo origina el ciclo que `solve(EqualityId)` detecta o el deber de constructor registrado con ese mismo ID que llega sin solución. Un deber previo de otro ID no se consume por compartir representante, span ni diagnóstico.
5. **Journal anidado.** Una hija compatible fusiona sus primeras escrituras y obligaciones creadas con el padre; una hija rechazada restaura solo las propias. El rechazo del padre restaura también las hijas fusionadas; `next_variable`, `FactId`, cuotas y trabajo permanecen monotónicos.

El `JoinRecord.first_incompatibility` de C3 reemplaza el mapa experimental `union_denounced`; registra el primer contraste incompatible por `ConsumerId` de la `if`. La unión sin segunda continuación no reserva E1303. En W0 la continuación `q` queda ligada a su ancla solo si el contraste publica; el rechazo conserva el E1303 propio y permite que la igualdad independiente `[r,q]` resuelva `r` después. En F06 C1/C2 la puerta exige una salida completa de `main` u oráculo; una salida parcial falla.

Pruebas dirigidas antes de puertas completas: los tres recursivos y `deferred_join_has_one_diagnostic_per_source_site`; `atomic_78`, W0 y W0nounion como controles cruzados de C3; F06 con firma completa; negativo de cuota E0003 y positivo sin unión. Ninguna prueba inventa un esperado desde el código bajo prueba.

## Inventario completo: fila propia → mecanismo mínimo → commit

Los IDs se transcriben de `r9-commit-gates.json` v12. `frozen13` repite IDs de `directed74`; se indica para que las tres listas queden cubiertas. Los grupos con intervalo son inclusivos y no omiten IDs.

| Familia y filas propias | Mecanismo mínimo exigido | Primer commit |
|---|---|---|
| dirigidas `recursive_69`, `recursive_78_equivalent`, `recursive_127_equivalent` | origen del constructor en el `Join` recursivo pendiente; sin journal nuevo | C1 |
| dirigida `atomic_78`; matriz `W0` | `JoinRecord` de la `if` y dependencias causales de su ancla; cierre del constructor consecuente | C3 |
| matriz `W0nounion` | control positivo gemelo de W0; conservar `checked` | C3 |
| dirigidas `B_M1`, `B_M2`, `planned`, `ST1_equivalent` | `EqualityRecord`, journal, contexto de entrada, origen por componente y completación | C2 |
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

Esta tabla es de **propiedad contractual**, no un PASS de ninguna fila del implementador. La referencia C1 ejecuta los tres recursivos y el cruce F06 sobre código sin R1–R5; el prototipo no fundamenta C1. La prueba comparativa de dos configuraciones se limita a `atomic_78`/`W0`/`W0nounion` y F06 del prototipo; las demás conservan las puertas v12 y las probará C con el código nuevo, D y D'.
