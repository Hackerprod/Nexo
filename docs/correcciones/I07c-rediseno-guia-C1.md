# I07c R6 — guía de implementación C1 para el cierre atómico

Estado: precisión de implementación de R6-final v11, **en validación**. No modifica el oráculo, los dueños de filas ni las puertas. B r17 revisará esta guía. El árbol de Luna continúa sin commit C1.

## Dictamen reproducible

Sí hay una **ablación funcional del mecanismo C1** en el prototipo: el commit `fc9600ab0cd7d8bbe56bd5eb28f2af907c034833` introdujo `NodeJournal` y su cierre en `anchor_contrast` (`src/check/types.rs`, diff de ese commit). Se compiló ese blob en release, SHA-256 del binario `82984d81d6ba93f3696f9461631002f0284ceac9f31a58195cae841b3077038b`, con rustc 1.99.1, `CARGO_BUILD_JOBS=4` y heavy-lock `i07c-rediseno`. Sobre las fuentes congeladas de `r8-directed74.json` y `r8-matrix45.json`, perfil explícito `f0-0002-b-v2`:

| Configuración del mismo binario | `atomic_78` | `W0` (firma completa) | `W0nounion` |
|---|---|---|---|
| `NEXO_XV=` | E0103 `[16,18)` + E1303 `[52,53)` | E0103 `[27,29)` + E1303 `[69,70)` | `checked` |
| `NEXO_XV=atomic0` | solo E1303 `[52,53)` | solo E1303 `[69,70)`, `Bool`/`Array<Array<Bool>>`, relaciones `[59,63)`/`[69,70)` | `checked` |

El mismo commit, sin o con `atomic0`, produce F06 **completo** según el oráculo. Su árbol contiene experimentos R1–R5 anteriores, incluso cambios incondicionales; por ello la ablación demuestra el mecanismo local de estos dos E0103, **no** que `fc9600a` sea un C1 íntegro sobre `31d91db`. Los primarios recursivos de ese blob aún difieren de los fijados por G3 en C0; `6e3eb03` porta el primario recursivo del prototipo. No se debe importar el árbol del prototipo ni sus flags.

La sonda adicional sobre copia desechable del WIP de Luna `96d5644` + `src/check/types.rs` sin commit cambió únicamente la guarda `preserve_failed_constructor` de E0103 **o** E1303 a solo E0103. Binario release SHA-256 `a1a4949ffc1fa308964542679e3147facb36b9f233bbb881b0d6b975437d25c5`: `atomic_78` y `W0` aún emiten los E0103 sobrantes, y F06 sigue parcial. Esta sonda descarta la mera sustitución de esa guarda. El cierre debe abarcar el **intento de ancla** que devuelve `Ok` con `Join` pendiente, no solo el `EqualityEnd` que recibe `Err` del solver.

## Operación que falta en C1

En `src/check/types.rs` de Luna, `InferenceGraph::{begin_equality,type_node_mut,commit_equality,reject_equality}` (aprox. 2879–3040), `Checker::solve`/`Frame::EqualityEnd` (aprox. 6124–7300) y los dos contrastes del ancla de `if` —`Checker::if_after_first` (aprox. 15060–15122) y `IfAction::SecondBranch` en `Checker::if_advance` (aprox. 15008–15052)— deben compartir **un cierre causal por `EqualityId`**. `Checker::if_finish_join` (aprox. 14687) recibe su estado; no decide por `span_key` si un constructor es independiente. En el prototipo, `anchor_contrast` (aprox. 14680 en `fc9600a`) abre el journal antes de `unify`, mide los `pending_joins`, restaura las primeras escrituras de nodos cuando el intento resulta pendiente o rechazado y conserva la obligación que producirá E1303. Ese `finish_node_journal(false)` es la operación que elimina el E0103 de `r`/`q`; no se filtra el diagnóstico al final.

El journal de Luna ya guarda primera escritura por `(node,campo)` y asignaciones. El cierre debe clasificar **`Ok` con Join nuevo** como `EqualityState::Waiting`: restaurar las escrituras tentativas (incluido `failed_constructor`, aristas, origen y padre) y conservar `EqualityRecord`, `Join` pendiente, `EqualityId`, ordinal y contadores monotónicos. Un `Ok` sin obligación nueva confirma; un `Err` rechaza y restaura. La reserva E1303 pertenece al `Join`/ancla, no al constructor fuente. Un E0103 independiente de un constructor anterior conserva su causa y su marca **previa**: la primera escritura del journal restaura el valor anterior, no borra todas las marcas del nodo. Si el E0103 nace del intento propio, se reserva por su `EqualityId` después de restaurar, una vez, con el constructor dueño. Nunca conservar ciegamente `FailedConstructor` por un E1303.

`dependencies: Set<EqualityId>` se calcula **al crear** el `EqualityRecord`, desde las igualdades publicadas que alcanzan los representantes/componentes de ambos operandos y desde las obligaciones anteriores que la nueva igualdad consume; se añade el ID a la obligación `Join` creada en ese intento. Se incrementa al crear una arista causal nueva, antes de cerrar el journal; no se reconstruye desde spans ni del tipo final. En C1 el `Join` histórico puede transportar el `EqualityId` como vista de transición; C4 migra esa relación a `ConstructorDuty.reaching_equalities` sin mantener dos autoridades. En `atomic_78`/`W0` basta esta relación directa y la restauración de la igualdad pendiente; `ReadContext`/`Completion` de C2 y el punto fijo de deberes de C4 no se adelantan. Una igualdad rechazada no puede publicar arista, origen ni marca E0103 de su constructor fuente. Una igualdad pendiente conserva su obligación y no publica esos campos. Dos causas independientes nunca se suprimen por compartir raíz, span o texto fuente.

Pseudocódigo operacional (la denominación final de tipos queda en C1):

```text
begin_anchor_attempt(left, right, owner, ordinal):
    id = equality_id(owner, ordinal)                 // C0; estable
    deps = reaching_published_equalities(left, right)
    record = EqualityRecord(id, left, right, deps, Waiting, journal())
    graph.begin_equality(record)
    before = graph.pending_join_count()
    result = solve_equality(id, left, right)          // journal primera escritura
    created = graph.pending_joins_since(before)
    for join in created: join.owner_equality = id     // conserva obligación
    if result is AnalysisLimit:
        graph.restore_journal(id)
        return result                                 // solo cuota real
    if result is Err:
        graph.restore_journal(id)
        graph.reserve_diagnostic(id, result)
        if result is own E0103: graph.mark_own_constructor(id)
        return Rejected(result)
    if created is not empty:
        graph.restore_journal(id)                     // también FailedConstructor
        graph.set_state(id, Waiting(created.ids))
        return Pending(id)
    graph.publish_journal(id)
    graph.set_state(id, Committed)
    return Compatible(id)

on_pending_join_resolution(join, outcome):
    id = join.owner_equality
    if outcome is incompatibility:
        reserve_one_E1303(join.owner, id, outcome)
        reject_unpublished_equality(id)
    else: publish_compatible_edges_once(id, outcome)
```

`graph.pending_joins_since(before)` debe identificar obligaciones por ID/ordinal, no solo por longitud si un cierre intermedio puede reordenarlas. Las escrituras de una igualdad hija confirmada se incorporan al journal padre; las de una hija rechazada se restauran antes de volver. `next_variable`, `FactId`, pasos y cuota nunca retroceden. Para `failed_constructor`, comprobar el valor antes de la primera escritura y la causa reservada después del cierre; no usar un booleano global `preserve_failed_constructor` para E1303.

## Cordura antes de la puerta completa

1. `atomic_78`: un solo E1303 `[52,53)`, sin E0103 `[16,18)`; `W0`: firma normativa completa de E1303 `[69,70)`, sin E0103 `[27,29)`; `W0nounion`: `checked`. Usar ambos perfiles tal como figuran en los JSON, sin normalizar spans.
2. Los tres recursivos conservan los primarios G3 de C0 (`[60,63)`, `[69,72)`, `[118,121)` según su fila) y los E0103 independientes; `deferred_join_has_one_diagnostic_per_source_site` conserva exactamente un E0103.
3. Repro pequeño de independencia: `fn Main(){var q=[];let z=true+1;}` mantiene E0103 del constructor `q` y E0102 de `true+1`. Gemelo con unión `if` fallida no puede añadir un E0103 al constructor que solo participó en esa unión. La regresión de cuota conserva `next_variable` monotónico y NX-E0003.
4. F06 sigue bajo la puerta cruzada v11 en C1/C2: salida **completa** de `main` o del oráculo, nunca la salida parcial comunicada por Luna. La ablación `atomic0` no prueba que el WIP actual cierre F06; si sigue parcial después de la corrección, C1 permanece bloqueado y se vuelve a A/B con el repro, sin excepción nueva.

## Inventario completo: fila propia → mecanismo mínimo → commit

Los IDs se transcriben de `r9-commit-gates.json` v11. `frozen13` repite IDs de `directed74`; se indica para que las tres listas queden cubiertas. Los grupos con intervalo son inclusivos y no omiten IDs.

| Familia y filas propias | Mecanismo mínimo exigido | Primer commit |
|---|---|---|
| dirigidas `recursive_69`, `recursive_78_equivalent`, `recursive_127_equivalent` | igualdad recursiva, journal, primario G3 ya portado en C0 | C1 |
| dirigida `atomic_78`; matriz `W0` | igualdad de ancla pendiente, rollback de primera escritura y reserva E1303 del intento | C1 |
| matriz `W0nounion` | unión ausente, no reservar conflicto ni contaminar constructor | C1 |
| dirigidas `B_M1`, `B_M2`, `planned`, `ST1_equivalent` | contexto de entrada, origen por componente y completación | C2 |
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

Esta tabla es de **propiedad contractual**, no un PASS de ninguna fila del implementador. La prueba comparativa de dos configuraciones se limita a `atomic_78`/`W0`/`W0nounion` y F06 del prototipo; las demás conservan las puertas v11 y las probará C con el código nuevo, D y D'.
