# I07c — respuestas y disposición tras el FAIL de B r1

Estado: propuesta de A, sin PASS de B. Ningún test existente se modifica aquí. El prototipo y los barridos están en `i07c-rediseno-proto`.

## Preguntas 7–18 del brief

| # | Respuesta y prueba que la debe cerrar |
|---|---|
| 7 | T5 literal: una `if` con condición fallida conserva los C de sus hijos y su contraste interno, pero no ofrece resultado ni hojas a la unión exterior. Texto y par válido/inválido por bytes en `I07c-rediseno-restauracion-atomica-lagunas-B-r1.md`. El ejemplo simple inválido da solo `NX-E0102` `[41,42)`; el ejemplo de B da `[69,70)`. |
| 8 | La forma inline `AssertEq([true],[m1[7],m1])` conserva los deberes independientes de los constructores `[]` `[17,19)` y `[m1[7],m1]` `[36,46)`; `[43,45)` es una lectura del mismo componente, no constructor nuevo (§4.1.2.2). El gemelo con `let` debe mantener códigos y cantidad de C en los constructores correspondientes, sin convertir el intermediario en otro C. El esperado exacto del gemelo se derivará antes de cambiar el test. |
| 9 | `skipv3` es un instrumento de aceptación, no regla de emisión: el cierre debe conservar cada `ConstructorDuty` independiente y suprimir solo el deber cuya única evidencia dependa de un resultado denunciado (§4.1.2.2, .5). El negativo de 69 bytes `var v=...;var a=[v];v=a;` sigue rechazado; la ubicación `[66,67)` requiere la precisión propuesta a B. Una lista global de errores omitidos hasta el fin no sustituye dependencias. |
| 10 | R1b (doble `NX-E1303` y testigo pendiente) y la fila B 4417061/2348 no se difieren: el libro reserva una sola denuncia por `JoinId` y conserva las igualdades compatibles de sus hojas (§4.1.2.4). B ya midió que 2g' cierra ambos; se usarán como regresiones dirigidas del rediseño, sin atribuir ese cierre a R5 hasta repetirlas. |
| 11 | S5a.5 permanece dentro del alcance, como exige brief §1. La procedencia A de `F0-0002-lagunas.md` se guarda al comprometer la igualdad; un enlace de completación posterior no sustituye el origen anterior. El A full de c1/2882 ya falsó esa propiedad en R5. La clase I permite revisión de instantánea tras validación independiente, no omitir la regla. |
| 12 | S5a.2: cada operando de `+`/`-` mantiene su igualdad I64 independiente aun si el otro falla; se emite una sola denuncia del dominio de la operación (§4.1.2.4). S5a.3: el resultado lógico contaminado no baja a IR; el negativo `fn Main(){let w=1+(true or false);}` del perfil 0005 debe dar diagnóstico de tipos, jamás `InvalidIr`. Son dos bloques funcionales del mismo rediseño, cada uno con positivo y negativo aislado; todavía no están prototipados. |
| 13 | La base contractual es la medición reproducida sobre el commit de partida con rustc 1.99.1, perfil test con debuginfo y `scripts/f5-frame-sizes.py`, no escoger 60.616 o 60.920 por preferencia. `Checker::drive` y `resume_non_match` se medirán juntos, además del exe/MAP; cada nueva familia entra por ayudante `#[inline(never)]`, con `FactId`, progreso y continuación explícitos. Parada si el crecimiento supera 2.500 B o el margen baja de 1.000 B sobre 66.560 B. **Medición pendiente.** |
| 14 | La copia de `type_nodes` de `atomic0` solo es instrumento. El mecanismo de producción será un journal por igualdad con claves de nodo escritas por la operación, más cierre transitivo de hijos Array/Option/Result y aliases Deferred que la igualdad toca; las obligaciones y marcas nacidas por esa igualdad tienen el mismo `OccurrenceId`. Rollback inverso solo de esas claves, sin clonar todo el grafo. El tamaño se acota por nodos/obligaciones alcanzables de los operandos de una igualdad, con cuota de solver; hay que medir el +10 % de tiempo antes de afirmar cumplimiento. |
| 15 | La cota propuesta para planificación es un orden parcial finito: cada ocurrencia pasa de no vista a pendiente y a decidida una vez por epoch de evidencia compatible; cada variable/construtor recibe una forma más determinada o un rechazo final, nunca vuelve a estado anterior. La cantidad de epochs se acota por igualdades fuente y la profundidad por la cuota del solver. Falta demostrar que el código concreto respeta monotonicidad; el tope 129 es barrera, no prueba. Cinco pasadas o más paran el diseño. |
| 16 | El orden de dos o tres asignaciones se decide por sus `OccurrenceId`: una igualdad rechazada no fija tipo ni retipa lecturas anteriores; una compatible se compromete en su ordinal. Se miden las 4.044 permutaciones y los mínimos `B_qr`/`B_rq` (§4.1.2.2, .7). |
| 17 | c1/c2 se complementan con 103 filas `match`, los corpus de B, cuatro corpus nuevos ya congelados (`c1-nueva`, `c2-nueva`, `c1-nueva2`, `c2-nueva2`) y los 9.060 nuevos de proyección independiente en la rama prototipo. Funciones propias, bucles, `Option`/`Result`, bloques `value`, anotaciones y `else if` se comprueban con 152 RD, 3.674 fuentes, twin y fuzz; el oráculo base no modela todas esas formas. Todo caso sin oráculo exige adjudicación por cláusula, nunca se cuenta como arreglo. |
| 18 | Cuotas: buscar el menor límite que acepta el programa control, exigir que el caso con obligación adicional rechace por cuota antes o en ese umbral, sin fijar 1..=8 ni 30/31. Auditor de spans: localizar función y expresión fuente por su identidad sintáctica y validar el span, sin número de línea fijo. Son andamiaje según `docs/F0_ACUERDOS.md` §10 y AGENTS.md; conservar ejemplos normativos y ausencia de pánico. |

## Disposición de los 17 tests de brief §3.6

La disposición de B en `I07c-S5c-tests-B-r1.md` se conserva. «Reescribir» significa hacerlo **después** de la derivación y de la aprobación individual exigida por brief §3.6, no en esta propuesta. Todos siguen sin tocar y no acreditan PASS de R5.

| Test (módulo abreviado) | Disposición |
|---|---|
| `causal::mixed_nested_join_keeps_constructor_and_shape_sites` | Reescribir: solo `NX-E1303` `[2,3]`. |
| `causal::adr_gap_nested_union_and_array_element_keep_current_causal_sites` | Reescribir: solo `NX-E1303` en `br`. |
| `causal::failed_outer_condition_keeps_nested_join_constructor` | Reescribir: quitar `NX-E0103` del vector. |
| `causal::failed_outer_array_keeps_nested_element_cause` | Dos diagnósticos `[25,27)` y `[49,52)`; el tercero del array exterior sigue INCONCLUSO. |
| `causal::pending_inner_constructor_preserves_outer_branch_comparison` | Lectura B: solo `NX-E0103` `[85,87)`. |
| `causal::unresolved_constructor_survives_failed_nested_union_condition` | Sustituir por gemelo de condiciones válidas y aplicar T5 al inválido; exacto del inválido sujeto a propuesta de laguna. |
| `mutation::anchored_first_branch_keeps_independent_pending_join` | Reescribir: solo `NX-E0102` `[136,137)`. |
| `mutation::linked_array_constructors_keep_each_causal_inference_site` | Caso 2: `[(80,82)]`. |
| `mutation::cyclic_constructor_errors_keep_inference_code_at_each_site` | Retirar solo `[43,45)` de la forma inline; gemelo `let` y negativo recursivo siguen INCONCLUSOS hasta precisión de ADR. |
| `pending::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` | Mantener cuatro diagnósticos derivados por B; la ablación `atomic0` no autoriza volver al esperado de `main`. |
| `pending::planned_projection_child_does_not_use_failed_branch_as_witness` | Mantener esperado de `main`/oráculo, `NX-E1303` en `q`; es sonda obligatoria. |
| `pending::failed_nested_union_does_not_anchor_an_outer_pending_array` | Reescribir: `NX-E1303` `[81,86)` y `NX-E0102` `[92,93)`. |
| `normative::independent_causal_sites_survive_deferred_resolution` | Caso `nested_constructor`: `[(28,30)]`. |
| `normative::deferred_projection_and_constructor_programs_finish_with_causal_results` | Caso `repeated_constructor_sites`: `[17,19)` y `[27,37)`. |
| `structural::grouped_operands_keep_one_causal_primary` | Auditor por función/expresión, no renumeración de línea. |
| `quota::deferred_if_join_charges_the_reduced_boundary` | Esperado relacional sobre frontera de cuota, sin cifra fija. |
| `quota::failed_branch_join_keeps_reduced_quota_error_prefix` | Esperado relacional, sin 30/31. |

`quota::i07c_survivor2_nested_shape_hints_reach_the_leaf` se conserva con su función de matador de M00123/M00125. La lista de defectos `F0-0002-I07c-defectos-abiertos.md` §§2.1–2.2 sigue abierta fila a fila; la matriz de 45 cubre varios mínimos pero no sustituye la adjudicación de FAM-E/FAM-O y todas las filas anidadas.

## Filas nominales de la lista de defectos

Esta disposición fija qué reproduce cada fila y qué comprobación falta; **no afirma que R5 la haya cerrado**. La fuente y salida esperada exactas son las de `docs/validation/F0-0002-I07c-defectos-abiertos.md` §§2.1–2.2 y los dos documentos de laguna de O; no se generan esperados a partir del prototipo.

| Filas | Disposición de arquitectura y validación pendiente |
|---|---|
| FAM-O O01, O03, O07, O08 | Ejecutar fuentes `reduced` completas además de sus mínimos en matriz 45; cotejar cada C, orden y ausencia de E0103 derivado. O08 comprueba T5 y conservación de los cuatro C interiores. |
| FAM-O O02, O04, O05, O06 | Ejecutar sus `reduced` completas con esperados de las propuestas O; O04–O06 ejercitan también S5a.2. O05 conserva la parte no derivada como INCONCLUSA, sin convertirla en PASS por desaparición de E0103. |
| FAM-E F01, F02, F04, F05 | Medir cada `reduced`: dos constructores pendientes y asignación compatible posterior; el rechazo de una igualdad no deshace la otra (§4.1.2.2). |
| FAM-E F07, F09, F10, F11 | F07/F10 por sus mínimos y fuentes con preludio; F11 por `B_qr` y permutación; F09 por `reduced`. Cotejar orden de asignaciones y hoja de proyección (§4.1.2.3, .7). |
| FAM-E F12, F13, F14, F15 | Cada `reduced`; F15 también mínimo y permutación. Contrastar hojas de unión y asignación posterior sin E0103 espurio (§4.1.2.2, .4). |
| FAM-E F16, F17, F18, F19 | Cada `reduced`; F16 mínimo con anotación, F18 primer C de unión todavía por derivar nominalmente antes del veredicto. F03 ya está CERRADA en la lista y F08 es control legítimo, no defectos abiertos. |
| FAM-2 F06 | Fuente `reduced` y mínimo F06 de §2.2 fila 5: E1303 en `r`, E0102 del `3`, sin E0103 ni traslado a `ar`. |
| Anidada 1–6 | A/A_noasg/A_reord, B_qr/B_rq, F06 y M2; cada orden de asignación y cada C de hoja, no solo el recuento. |
| Anidada 7–13 | R4, X_R1na, X_R1ord, X_flat3, Y_pend_if/2 y X_R1inc; una denuncia por unión propia, ancla de primera continuación y conservación de asignación posterior. |
| Anidada 14–20 | X_P03/04/05/15/16/17 y W0; distinguir unión denunciada, testigo parcial y constructor propio. |
| Anidada 21–25 | P1/c3/w1, P2/w3, P3, P4pos y readplan; comprobar la relación de procedencia A, que la asignación rechazada no retipa y que `read_plan` no adelanta un contexto. P4pos y readplan exigen oráculo manual porque el oráculo base no modela su fuente. |

Cada una de esas filas se marcará PASA/FALLA/INCONCLUSO con salida esperada y obtenida después del barrido del binario fijado por SHA. La agrupación por causa no elimina la obligación de una fila individual.
