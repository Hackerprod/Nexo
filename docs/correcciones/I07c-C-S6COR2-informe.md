# I07c S6-COR-2 - informe de C (implementador)

**Estado: EN VALIDACION, no PASS.** Rama local `i07c-s6cor2` (worktree `C:\projects\Nexo-i07c-b`), partiendo de `origin/main` = `a24a1a1`. Sin push. Plan: `docs/correcciones/I07c-S6COR2-plan.md` (rama `origin/i07c-estructural-r2`, HEAD `0501d5b`, PASS de B en r5).

## Nota de proceso (desvio por presupuesto)

Implementado por un agente Sonnet de la sesion del coordinador por **desvio por presupuesto** (fondo de Codex agotado). Re-validable con Codex tras el 2026-11-24. Se hizo la mutacion de las regiones tocadas con poblacion congelada (obligatoria por el desvio). D y D' deben ser agentes nuevos.

## Commits (uno por solucion; todos con "EN VALIDACION, no PASS")

| Commit | Contenido |
|---|---|
| S0 `9cc5407` | andamiaje del auditor `scripts/i07c_span_audit.py`: baseline congelado por (archivo, ancla del sitio = funcion + brazo envolvente, expresion) -> recuento para `lower.rs` (85) y `lower_match.rs` (23); registro `GROUP_SITES` de anclas positivas; extension de `--self-test`; una linea en `tests/i07c_structural_invariants.rs` (`lowering span baseline: PASS`) |
| S0 pruebas `4c734da`, `40aba0b`, `937f246` | infraestructura de regresion de cuotas en `tests/i07c_group_primary.rs` (plantilla con pad y envolturas, esperados derivados de la fuente y del control sin grupo, barrido de pad 0..=11) y su consumidor sobre un sitio ya correcto (inicializador de `let`). `937f246` solo anade el `use accepted` que faltaba al commit `4c734da` (error mio: ese commit intermedio no compila; el arbol de `937f246` en adelante si) |
| S1 `049b48f` | `ExpressionStatement`: `parse::expression_primary_span(value)` |
| S2 `c07b8f0` | `WhileCondition`: `parse::expression_primary_span(condition)` |
| S3 `2d47cc2` | `Assignment`: `parse::expression_primary_span(place)` |
| S4 `f7a5a9d` | `ValueStatement` (operando sin `value:`) y su espejo en `statement_leaf` |
| S6 `8002952` | `guard_violations`/`Assign`: `guard_mutation` con `expression_primary_span(place)` |
| S5 `542d5b1` | `prepare_expr`: `let callee = parse::expression_primary(target)`; el destino agrupado cobra el tick de nombre |
| S7 `33046c4` | `guard_violations`/`Call`: `Drop` reconocido y senalado sin agrupaciones exteriores |
| S8 `b288681` | solo pruebas: `guard_checks_reach_every_nested_edge` (ver "Mutacion") |

Produccion tocada solo en `src/ir/lower.rs` y `src/ir/lower_match.rs`. No se toco `src/check/types.rs`, `tests/f0005_b1b.rs`, `tests/fixtures/i07c_rd.json`, `corpus/`, `docs/baseline/`, `docs/adr/`.

## Por solucion: regresion que falla sin ella

Cada regresion se comprobo en rojo con la correccion revertida y en verde con ella; esperados derivados de la fuente (posicion del texto interior en la plantilla) y del control sin grupo, no del binario (`tests/i07c_group_primary.rs`).

- **S1** (6 tests `expression_statement_tick_*`: `n+1`, `Id(n)`, `AssertEq(1,1)`, `fn (){}`, `()`, `[1]`): sin la correccion el primario es `(n+1)`.
- **S2** (`while_condition_tick_*`, incluida la fila `n<9999999` del plan: base `[30,41)`, esperado `[31,40)` con una agrupacion).
- **S3** (`assignment_tick_reports_the_inner_place`: base `(33,36)`, esperado `(34,35)`).
- **S4** (`then_operand_*`, `else_operand_*`, `branch_operand_*`). El espejo de `statement_leaf` (no observable) queda sin regresion y exento de mutacion inversa, solo con su ancla positiva.
- **S5** (`grouped_call_target_charges_the_ticks_of_the_bare_call`, `grouped_closure_call_*`): relacional sobre la frontera de la cuota; calcula N por biseccion con el control (43.477 en `Id(1)`, coincide con el plan, sin fijar la cifra) y exige que `(Id)(1)`, `((Id))(1)` y `( Id )(1)` completen N y fallen en N+1 con el mismo R0001 (spans movidos por la envoltura). Sin la correccion: "43478 must fail under (...)".
- **S6** (CASES `guard_mutation_*`, `check` y `run`): sin la correccion `(52,55)` frente al esperado `(53,54)`.
- **S7** (CASES `guard_transfer_*`; `grouped_callee_other_than_drop_is_accepted_in_a_guard`): sin la correccion `(Drop)(a)` se acepta (exit 0).

Adaptacion respecto del plan (no es cambio de produccion): los casos de cuota son solo de `run` (el `check` los acepta, es un hecho de ejecucion) y el `Case` de `CASES` exige rechazo en `check` y `run`; por eso S1-S5 van en tests dedicados con tabla `QuotaSite` (un bloque `// Sn` cada uno) y S6/S7 en `CASES` (`// S6`, `// S7`). El ayudante `moved_span` se ajusto en el commit S4 (un span que contiene al interior conserva su inicio: el `If` de la rama `else`); es un ajuste de las pruebas de S0 descubierto al probar S4.

## Tests existentes y cifras protegidas

- **Tests existentes cambiados: ninguno** (el diff de `tests/` solo anade: `i07c_group_primary.rs` y una linea de asercion nueva en `i07c_structural_invariants.rs`).
- **Cifras protegidas 2/6/1 y 56/25/15, R0001/R0002 de `src/ir/i10b_tests.rs`, `work_oracle_tests.rs`, `tests/*` nombrados, `corpus/` y `corpus/objetivo/`: ninguna cambia** (la suite completa pasa sin tocar esperados; el diferencial sobre `corpus/` y `tests/fixtures/` (217 `.n`) solo muestra las diferencias de la tabla de abajo, ninguna en R0001/R0002 de `corpus/`). No se activo la PARADA.

## Pila

`scripts/f5-stack65.py` (candidato, rustc 1.99.1): `option_match126` y `result_match126` a 65 KiB 5/5 exit 0, control a 16 KiB `0xc00000fd`. `scripts/f5-frame-sizes.py` con `/MAP` (ejecutable lib SHA-256 `d5afd820...0781`, `.map` `a9c84a67...ceaf`): `drive` 27.536 B, `resume_non_match` 33.080 B, **pareja 60.616 B, margen +5.944 B**: identico a la linea base vigente (antes = despues; `types.rs` no se toca).

## RD

`python -B scripts/i07c_rd_conformidad.py --verify <nexo.exe debug del candidato>`: `RD normative: 152/152 check/run pairs`, `RD snapshot differences: 0`.

## Suite, fmt y clippy

`cargo nextest run --profile default --offline --no-fail-fast` en tres tandas (`--partition count:k/3`): debug 730+729+729 = 2.188 pasan, 0 fallos, 0 timeouts; release 731+730+729 pasan. `cargo fmt --check` y `cargo clippy --offline --all-targets -- -D warnings` limpios. `python -B scripts/i07c_span_audit.py --self-test`: PASS (25 reversiones de sitio rechazadas). La puerta completa (`validate.sh`/`gate.sh`) la lanza el coordinador; no la ejecute.

## Diferencial base `a24a1a1` frente al candidato (produccion = `33046c4`; `b288681` solo anade una prueba)

Corpus **congelado antes de ejecutar** (`docs/validation/i07c-s6cor2/FREEZE.json`, `MANIFEST.txt`, generador `gen_corpus.py`): **5.685 fuentes**, semilla `20261122`, SHA-256 del manifiesto `703dfdf8dfbd17a744de31373b15b7da027043318aff2bd1b456cabe7938d0bd`. Contenido: 25 familias de sitios (sentencia de expresion, asignacion, condicion, operandos, indices, destinos, guardas, `for`, etc.) x envolturas (ninguna, `( )`, `(( ))`, `( e )`) x pad 0..11 (bucle) y 0,1 (bloque); fronteras de cuota con destino agrupado (18 valores de N); 700 composiciones aleatorias con semilla; los 217 `.n` de `corpus/` y `tests/fixtures/`; las 152 filas RD.

Ejecucion: `check` y `run`, perfiles `f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`; **release: las 5.685 fuentes** (34.110 celdas); **debug: 1.778 fuentes** (todas las que no contienen `while` + 1 de cada 8 del orden alfabetico; 10.668 celdas). **Limite declarado:** el debug no cubre todo el corpus porque un `run` en debug que agota la cuota de 1.000.000 ticks tarda ~4 s (el corpus completo en debug eran ~4 h). Debug frente a release: **0 diferencias** en base y en candidato (en las 1.778). Resultados comprimidos: `results-release.jsonl.gz`, `results-debug.jsonl.gz`.

**Diferencias base/candidato: 1.248 celdas (1.080 en release, 168 en debug), todas listadas una a una en `docs/validation/i07c-s6cor2/differences.json` (fuente, celda, modo, regla, motivo con los primarios base y candidato)**; clasificadas por regla por `diff_classify.py` a partir de la fuente y de los dos diagnosticos (0 `UNCLASSIFIED`):

| Regla | Celdas | Derivacion |
|---|---|---|
| S1-S4: primario de R0001 sin agrupaciones exteriores | 524 | mismo diagnostico salvo el span; el primario base empieza por `(` y el del candidato es el resultado de quitar las agrupaciones exteriores del texto base (`()` unidad no es agrupacion): addendum "Primario de una expresion bajo agrupaciones" + "Ticks, cuotas y guardas bajo agrupaciones"; F0-0002 11 "Origen de los ticks" |
| S5: destino agrupado cobra el tick de nombre | 300 | solo en fuentes con destino agrupado (`(Id)(`, `(f)(` con cualquier agrupacion/trivia); ambos R0001 (primario movido por un tick) o base `unit` y candidato R0001: nunca el candidato llega mas lejos que la base; F0-0002 11 "Los parentesis de agrupacion no crean nodos cobrables" y "Nombre del destino de una llamada directa: 1" |
| S6: primario de `guard_mutation` | 212 | como S1-S4 (primario base `(x)`, candidato `x`), `NX-E1305/guard_mutation`, F0-0003 3.3 |
| S7: `(Drop)(a)` en una guarda | 212 | el candidato da `NX-E1305/guard_transfer` con primario `Drop` y la base no lo da (aceptaba, o seguia a un `NX-E0202` posterior); el control `Drop(a)` ya lo daba |

Comprobacion independiente de la regla de fondo ("la agrupacion no cambia aceptacion ni ejecucion"): en el candidato, **las 20.772 celdas de variantes agrupadas de las familias a_ y b_ tienen el mismo exit que su variante sin grupo**; en la base 66 celdas lo violan (S5/S7). Las variantes sin grupo (`w0`) no aparecen en la tabla de diferencias (sin diferencias en programas sin grupo).

## Mutacion inversa por solucion (copia desechable `.scratch/mut`, perfil `default` completo, fail-fast, debug luego release)

Parche = `parse::expression_primary*` -> acceso crudo (en S5/S7, `parse::expression_primary(target)` -> `target.as_ref()` para que compile). SHA-256 del blob inicial `f8fcbfde...74a5` (`lower.rs`) / `609641ce...2f26` (`lower_match.rs`); restauracion verificada (`sha_restored` = inicial) en las 7:

| Sol. | SHA-256 mutado (prefijo) | MUERTO por |
|---|---|---|
| S1 | `08bf6429` | `i07c_group_primary::expression_statement_tick_reports_the_inner_array` |
| S2 | `b11a37f5` | `i07c_group_primary::while_condition_tick_reports_the_inner_comparison_of_a_large_bound` |
| S3 | `2fa7d25c` | `i07c_group_primary::assignment_tick_reports_the_inner_place` |
| S4 | `949e6fb7` | `i07c_group_primary::branch_operand_tick_reports_the_inner_expression` |
| S5 | `763ac7db` | `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` (primer fallo de la suite; la regresion propia `grouped_call_target_charges_...` fallaba con el mismo comportamiento en el rojo previo a la correccion) |
| S6 | `a5a62850` | `i07c_group_primary::guard_mutation_reports_the_inner_place` |
| S7 | `c3ecc734` | `i07c_group_primary::guard_transfer_reports_the_inner_callee` |

El espejo de S4 queda exento (solo ancla positiva; su reversion la rechaza `--self-test`). Los SHA-256 completos estan en los JSON de la campana (`.scratch`, no versionados; reproducibles con `mut_patches.py`).

## Mutacion de las regiones tocadas (poblacion congelada)

Poblacion: **69 mutantes** (`docs/validation/i07c-s6cor2/mutation-population.json`, SHA-256 `f20d98ca816f2c3aaedf4ef8143e69be1377e536a47d5b7487092c8297046b8b`, congelada antes de ejecutar; operadores de `scripts/i07c_mutaciones.py` sobre `prepare_expr` y `guard_violations` completas y +-15 lineas alrededor de los cuatro sitios de tick de `lower.rs`). Veredicto MUERTO por primer fallo, SUPERVIVIENTE con la suite completa en debug y release (default, retries 0). Primera pasada: **36 muertos, 5 invalidos (no compilan), 28 supervivientes**. Los supervivientes de `guard_violations` eran supresiones de descenso; anadi la regresion S8 (`guard_checks_reach_every_nested_edge`: 25 filas con un `Drop(a)` o un modo de seleccion tras cada arista alcanzable, esperados = posicion del token en la fuente) y relance los 23 supervivientes de `lower_match.rs` contra ella (y M00041 tras anadir la fila `value:`): **15 pasan a MUERTO** (solo debug, test dirigido: el mutante muere con la nueva regresion).

**13 supervivientes registrados (deuda con fecha 2026-11-23, para `Pending.md`; revisar en la siguiente campana de mutacion sobre el codigo vigente), sin programa aportado que exhiba comportamiento erroneo:**

- `lower_match.rs`: M00038 (descenso al valor de `return` en una guarda: `return` da `NX-E0006` antes), M00047/M00048 (descenso en cuerpo e iterable de `for` en guarda: `NX-E0006`/`NX-E0002` antes del lowering), M00061 (descenso al destino de una llamada con expresion: `NX-E0006` antes), M00040 (descenso al valor de un `GuardControl`), M00051 (descenso a la guarda de un `match` anidado: ese `match` ya se rechaza por su propia guarda), M00054 (`&&`->`||` en la deteccion del `?` postfijo en guarda), M00057 (descenso al operando de `move`/`edit` en guarda). Codigo preexistente de `guard_violations` que S6/S7 no tocan.
- `lower.rs`: M00006 (`borrow: false`->`true` en una rama del `let`), M00013 (supresion de `saved_emission` en el marcador de valor), M00021/M00023 (guardas de rutas que terminan en `InvalidIr`, no observables), M00029 (busqueda del token del `Unary` en `prepare_expr`: afecta al origen del tick del token de `move`/`edit`; no se aporto programa).

Los 5 invalidos: M00002, M00016, M00018, M00024, M00030 (`||` de clausura mutado a `&&`, o `continue`->`break` fuera de bucle: no compilan).

Tabla completa:

| id | sitio | operador | linea mutada (original) | veredicto | mata |
|---|---|---|---|---|---|
| M00001 | lower.rs:7697 | eq | if self.emission == EmissionMode::Live { | MUERTO | ir::i10b_tests::boundary_mutation_witnesses_cover_condition_statement_and_index_preorder_origins |
| M00002 | lower.rs:7703 | or | let prepared = (// -> Result<AssignState<'_>, CompileError> { | INVALIDO (no compila) |  |
| M00003 | lower.rs:7709 | eq | if self.locals[local.0].parameter_mode == Some(ParameterMode::Read) { | MUERTO | check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00004 | lower.rs:8148 | false | borrow: false, | MUERTO | interp::machine::tests::indexed_array_growth_uses_exact_cached_cell_count |
| M00005 | lower.rs:8154 | eq | if self.emission == EmissionMode::Live { | MUERTO | interp::machine::tests::repeated_borrowed_index_releases_consumed_index_cells |
| M00006 | lower.rs:8167 | false | borrow: false, | SUPERVIVIENTE (registrado) |  |
| M00007 | lower.rs:9627 | increment | self.dead_depth += 1; | MUERTO | ir::effects::tests::ownership_liveness_propagates_across_cfg_predecessors |
| M00008 | lower.rs:9628 | eq | if entry_emission == EmissionMode::Live { | MUERTO | ir::i10b_tests::boundary_mutation_witnesses_cover_condition_statement_and_index_preorder_origins |
| M00009 | lower.rs:9824 | eq | if frame.index == frame.statements.len() { | MUERTO | frontend::tests::multi_token_construct_comment_gaps_preserve_execution |
| M00010 | lower.rs:9827 | drop_assignment | value_tail.started = true; | MUERTO | frontend::tests::multi_token_construct_comment_gaps_preserve_execution |
| M00011 | lower.rs:9827 | true | value_tail.started = true; | MUERTO | frontend::tests::multi_token_construct_comment_gaps_preserve_execution |
| M00012 | lower.rs:9828 | drop_assignment | value_tail.saved_termination = self.terminated.take(); | MUERTO | ir::i07a_tests::value_after_return_is_checked_in_dead_branch |
| M00013 | lower.rs:9829 | drop_assignment | value_tail.saved_emission = self.emission; | SUPERVIVIENTE (registrado) |  |
| M00014 | lower.rs:9831 | drop_assignment | self.emission = EmissionMode::Analyze; | MUERTO | ir::i07a_tests::value_immediately_after_return_is_checked_without_emission |
| M00015 | lower.rs:9833 | eq | if self.emission == EmissionMode::Live { | MUERTO | ir::f5_operator_tests::logical_short_circuit_uses_verified_cfg_and_charges_only_the_taken_rhs |
| M00016 | lower.rs:9836 | or | value_tail.marker.cloned().unwrap_or_else(// { | INVALIDO (no compila) |  |
| M00017 | lower.rs:9847 | false | borrow: false, | MUERTO | ir::i07c_tests::branch_join_uses_the_concrete_type_acquired_during_later_continuation |
| M00018 | lower.rs:9849 | continue | continue; | INVALIDO (no compila) |  |
| M00019 | lower.rs:10020 | ne | if flow.ty() != Some(&self.result) && self.dead_depth == 0 { | MUERTO | ir::effects::tests::return_exit_may_start_at_the_return_terminator |
| M00020 | lower.rs:10020 | and | if flow.ty() != Some(&self.result) && self.dead_depth == 0 { | MUERTO | ir::effects::tests::return_exit_may_start_at_the_return_terminator |
| M00021 | lower.rs:10020 | eq | if flow.ty() != Some(&self.result) && self.dead_depth == 0 { | SUPERVIVIENTE (registrado) |  |
| M00022 | lower.rs:10023 | drop_assignment | self.terminated = Some(Termination::Return(temp)); | MUERTO | ir::effects::tests::explicit_return_exit_spans_match_their_own_ast_returns |
| M00023 | lower.rs:10031 | eq | if self.emission == EmissionMode::Live { | SUPERVIVIENTE (registrado) |  |
| M00024 | lower.rs:10037 | or | .unwrap_or_else(// parse::expression_primary_span(value)), | INVALIDO (no compila) |  |
| M00025 | lower.rs:10049 | false | guarded: false, | MUERTO | ir::lower::tests::f5_array_for_indexed_place_keeps_original_root_borrowed |
| M00026 | lower.rs:10238 | eq | if self.emission == EmissionMode::Live { | MUERTO | ir::f5_operator_tests::compound_scalar_place_has_one_exclusive_loan_and_t08_work |
| M00027 | lower.rs:10250 | drop_state | own.push(span.clone()); | MUERTO | interp::machine::tests::repeated_borrowed_index_releases_consumed_index_cells |
| M00028 | lower.rs:10253 | drop_state | own.push(callee.span().clone()); | MUERTO | interp::machine::tests::repeated_borrowed_index_releases_consumed_index_cells |
| M00029 | lower.rs:10264 | eq | .find(/token/ token.span.start == span.start) | SUPERVIVIENTE (registrado) |  |
| M00030 | lower.rs:10265 | or | .map_or_else(// span.clone(), /token/ token.span.clone()); | INVALIDO (no compila) |  |
| M00031 | lower.rs:10266 | drop_state | own.push(token); | MUERTO | j14_accounting::published_new_operation_work_totals |
| M00032 | lower.rs:10274 | drop_state | own.push(span.clone()); | MUERTO | ir::i10b_tests::indexed_assignment_delta_and_result_error_include_semantic_charges |
| M00033 | lower_match.rs:105 | drop_state | frames.push(Frame::Statement(statement)); | MUERTO | i07c_group_primary::guard_mutation_reports_the_inner_place |
| M00034 | lower_match.rs:110 | drop_state | frames.push(Frame::Expr(value)); | MUERTO | i07c_group_primary::guard_transfer_reports_the_inner_callee |
| M00035 | lower_match.rs:113 | drop_state | found.push(("guard_mutation", parse::expression_primary_span(place))); | MUERTO | i07c_group_primary::guard_mutation_reports_the_inner_place |
| M00036 | lower_match.rs:114 | drop_state | frames.push(Frame::Expr(value)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00037 | lower_match.rs:115 | drop_state | frames.push(Frame::Expr(place)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00038 | lower_match.rs:127 | drop_state | frames.push(Frame::Expr(value)); | SUPERVIVIENTE (registrado) |  |
| M00039 | lower_match.rs:133 | drop_state | found.push(("guard_control", token_span.clone())); | MUERTO | j05_match_guards::break_continue_and_yield_inside_guard_use_control_diagnostic |
| M00040 | lower_match.rs:135 | drop_state | frames.push(Frame::Expr(value)); | SUPERVIVIENTE (registrado) |  |
| M00041 | lower_match.rs:140 | drop_state | frames.push(Frame::Expr(value)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00042 | lower_match.rs:150 | drop_state | frames.push(Frame::Statement(other)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00043 | lower_match.rs:152 | drop_state | frames.push(Frame::Block(then_block)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00044 | lower_match.rs:153 | drop_state | frames.push(Frame::Expr(condition)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00045 | lower_match.rs:158 | drop_state | frames.push(Frame::Block(body)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00046 | lower_match.rs:159 | drop_state | frames.push(Frame::Expr(condition)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00047 | lower_match.rs:162 | drop_state | frames.push(Frame::Block(body)); | SUPERVIVIENTE (registrado) |  |
| M00048 | lower_match.rs:163 | drop_state | frames.push(Frame::Expr(iterable)); | SUPERVIVIENTE (registrado) |  |
| M00049 | lower_match.rs:171 | eq | if selection.mode == Some(Mode::Consume) { | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00050 | lower_match.rs:180 | drop_state | frames.push(Frame::Block(&arm.body)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00051 | lower_match.rs:182 | drop_state | frames.push(Frame::Expr(guard)); | SUPERVIVIENTE (registrado) |  |
| M00052 | lower_match.rs:185 | drop_state | frames.push(Frame::Expr(&selection.scrutinee)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00053 | lower_match.rs:193 | eq | if span.start == value.span().start && span.end > value.span().end => | MUERTO | j05_match_guards::q06_try_cannot_exit_guard |
| M00054 | lower_match.rs:193 | and | if span.start == value.span().start && span.end > value.span().end => | SUPERVIVIENTE (registrado) |  |
| M00055 | lower_match.rs:200 | eq | let token = if cause == "guard_control" { | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_fits_small_worker |
| M00056 | lower_match.rs:213 | drop_state | found.push((cause, token)); | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_fits_small_worker |
| M00057 | lower_match.rs:215 | drop_state | frames.push(Frame::Expr(value)); | SUPERVIVIENTE (registrado) |  |
| M00058 | lower_match.rs:219 | eq | if matches!(callee, parse::Expr::Name(name, _) if name == "Drop") { | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_reports_transfers_in_source_order |
| M00059 | lower_match.rs:220 | drop_state | found.push(("guard_transfer", callee.span().clone())); | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_reports_transfers_in_source_order |
| M00060 | lower_match.rs:223 | drop_state | frames.push(Frame::Expr(&arg.value)); | MUERTO | j05_match_guards::invalid_guard_does_not_hide_independent_ownership_errors |
| M00061 | lower_match.rs:225 | drop_state | frames.push(Frame::Expr(target)); | SUPERVIVIENTE (registrado) |  |
| M00062 | lower_match.rs:229 | drop_state | frames.push(Frame::Expr(item)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00063 | lower_match.rs:236 | drop_state | frames.push(Frame::Expr(right)); | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_reports_transfers_in_source_order |
| M00064 | lower_match.rs:237 | drop_state | frames.push(Frame::Expr(left)); | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_reports_transfers_in_source_order |
| M00065 | lower_match.rs:240 | drop_state | frames.push(Frame::Expr(index)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00066 | lower_match.rs:241 | drop_state | frames.push(Frame::Expr(target)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00067 | lower_match.rs:255 | drop_state | frames.push(Frame::Block(then_block)); | MUERTO | i07c_group_primary::guard_mutation_reports_the_inner_place |
| M00068 | lower_match.rs:256 | drop_state | frames.push(Frame::Expr(condition)); | SUPERVIVIENTE -> MUERTO por S8 | guard_checks_reach_every_nested_edge |
| M00069 | lower_match.rs:277 | eq | found.dedup_by(/(_, left), (_, right)/ left.start == right.start); | MUERTO | ir::lower::lower_match::guard_traversal_tests::nested_guard_reports_transfers_in_source_order |

## Auditor: anclas, baseline y autotest

`lower.rs`: baseline 85 -> 80 accesos crudos (S1-S4 retiran los suyos; S5 pasa a `callee.span()`, ya sin grupos); `lower_match.rs` 23 -> 22 (S6 retira; S7 pasa a `callee.span()`). 8 anclas positivas (S1-S7 + espejo de S4). `--self-test`: rechaza expresion nueva, recuento mayor, recuento menor (baseline obsoleto) y la reversion de cada ancla; acepta el mismo uso en un modulo de test.

## Limites y hallazgos

- **Sin cambio de nucleo no previsto: no se escribio `...-hallazgo.md`.** Todos los cambios de produccion son los de la seccion 4 del plan.
- Debug del diferencial sobre un subconjunto determinista (arriba); el release cubre todo.
- Las anclas del auditor dependen de que `rustfmt` mantenga la forma de las llamadas en esos sitios (regex con `\s*`); un cambio de formato exigiria ajustar la regex del registro.
- Versionado en `docs/validation/i07c-s6cor2/`: `FREEZE.json`, `MANIFEST.txt`, `gen_corpus.py`, `diff_run.py`, `diff_join.py`, `diff_classify.py`, `differences.json`, `results-*.jsonl.gz`, `mutation-population.json`, `mut_population.py`, `mut_patches.py`, `mut_one.py`.
