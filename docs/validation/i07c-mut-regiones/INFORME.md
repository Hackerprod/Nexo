# I07c - mutacion de las regiones de S6-COR y S5b.1 (informe de C)

> Actualizacion 2026-11-24: la campana continuo con ejecutor paralelo (`run_par.py`) y, tras agotar el limite de commit de Windows (mutante con asignacion sin limite, ver `memguard.py` y `results-s5b1-memoria.jsonl`), con Job Objects y vigilancia de commit. Los supervivientes posteriores al id de la primera tanda NO tienen caza todavia (deuda con fecha 2026-11-24, ver la columna de nota).

**Estado: EN VALIDACION, no PASS.** Rama local `i07c-mut-regiones` (worktree `C:\projects\Nexo-i07c-f`), partiendo de `origin/main` = `68e0d2b`. Sin push. No se toco codigo de produccion: solo `docs/validation/i07c-mut-regiones/` y tres regresiones en `tests/`.

## Nota de proceso (desvio por presupuesto)

Ejecutado por un agente Sonnet por **desvio por presupuesto** (fondo de Codex agotado). Re-validable con Codex tras el 2026-11-24 04:43 (los mutantes pendientes y la caza de los supervivientes sin programa).

## Resultado resumido

| Corte | Poblacion | Ejecutados | Muertos | No compilan | Supervivientes | Inconclusos en resultados | Inconclusos a repetir (en `-inconclusos`) |
|---|---|---|---|---|---|---|---|
| S6-COR | 400 (de 1.313) | 279 | 181 | 48 | 47 | 3 | 3 |
| S5b.1 | 400 (de 1.048) | 273 | 138 | 55 | 70 | 10 | 15 |

**Cobertura incompleta, declarada**: de los 800 mutantes congelados se ejecutaron 552 (279 de S6-COR y 273 de S5b.1) (el orden de ejecucion estaba fijado ANTES de ejecutar por barajado con semilla, campo `order` de cada poblacion: son una muestra aleatoria sin sesgo y ninguna seleccion dependio de resultados). Quedan **248 pendientes**, nominalmente los ids sin fila en las tablas (lista al pie de cada tabla). Motivo: tope de ~6 h y carga de la maquina compartida: otro carril uso el cerrojo pesado de forma casi continua (puertas y validadores), de modo que el ritmo medio fue de unos 6 a 20 mutantes por hora, y cada superviviente cuesta la suite completa en debug y release (~6 min). Se ejecutan con `run_mut.py` sobre las mismas poblaciones (SHA-256 abajo); sin una sesion con maquina dedicada, los 600 restantes necesitan del orden de 30 a 40 h.

**Hallazgos**: tres defectos de cobertura con programa (M00158, M00156, M00067), cada uno con regresion que falla con el mutante y pasa sin el; un mutante equivalente por argumento de invariante de codigo (S5b.1 M00062); un mutante detectado por no terminacion (S5b.1 M00168); el resto de supervivientes, sin programa tras la caza. Los programas aparecen solo con fuentes dirigidas (error de sintaxis en un brazo con guarda; `break` tras un `match` con guarda; `break` tras un `for` con iterable rechazado): los corpus de diferenciales existentes (S6-COR-2: 5.685 fuentes; S5b.1: 3.000 selectores) no los contienen, lo que muestra su limite para regiones de parser y de control de bucles.

## Metodo

- **Region = diff por funcion.** Lineas de produccion de `HEAD` atribuidas por `git blame` a los commits de cada corte (S6-COR: `1010938..2bc5c8d` en `src/frontend/parse.rs`, `src/ir/effects.rs`, `src/check/names.rs` y `src/check/types.rs`; S5b.1: `4acee20..1010938` en `src/check/types.rs`, commits `6e9623e` y `1010938`), **expandidas a cada `fn` completa que las contiene** (por lineas sueltas solo salian 33 mutantes en S6-COR). Se excluyen `#[cfg(test)]` y comentarios (mascara de `scripts/i07c_mutaciones.py`). `lower.rs` y `lower_match.rs` NO se mutan aqui: los cubre la campana de S6-COR-2 (69 mutantes) y `lower.rs` cambio despues; lo tocado por S6-COR en `lower*.rs` fuera de esas regiones queda sin campana (limite).
- **Operadores**: los de `scripts/i07c_mutaciones.py` (`==`/`!=`, `<`/`<=`, `&&`/`||`, `true`/`false`, `continue`/`break`, `+=1`/`-=1`, eliminar `record_error`/`push`/`insert`/asignaciones, intercambio de `primary_span(ordered(n))` y de argumentos de `unify`) mas `EXTRA` de `mut_population.py` (`>`/`>=`, `+`/`-`, `.start`/`.end`, `is_some`/`is_none`, `is_ok`/`is_err`, `min`/`max`, `first`/`last`, `any`/`all`, constantes enteras +-1, negacion de condiciones de `if` y guardas de `match`, eliminar `return`, intercambio de argumentos de llamadas de dos argumentos).
- **Congelacion** (commit `30e23c0`, anterior a toda ejecucion): `population-s6cor.json` SHA-256 `659de53139e83c5d98bd6d9b4ed9f5bac084942a1b512e2fa832acdcd027fb13`; `population-s5b1.json` SHA-256 `ed0489c4b25743114e300350059366fa34cc429ee2027d3fc40117a18b6d2e0e`. Tope 400 por corte (enumeracion completa: S6-COR 1.313, S5b.1 1.048), muestreo determinista `random.Random(20261123)`, orden `random.Random(20261124)`. Blobs de produccion de `HEAD` en la poblacion (`blob_sha256`).
- **Ejecucion**: `run_mut.py` (reutiliza `scripts/mutant-verdict.py`), un mutante cada vez en copia desechable `C:\nexo-work\val\mut-regiones\tree` (`git archive`), target propio `C:/nexo-work/target-mut-regiones`, `cargo nextest run --locked --offline --profile default --retries 0` (suite completa, parada en el primer fallo); debug primero; MUERTO = primer test fallido (nombrado); no compila = NO COMPILA; debug verde -> release completa; ambos verdes = SUPERVIVIENTE. SHA-256 inicial, mutado y restaurado por mutante (el ejecutor afirma `sha_restored == sha_initial`); al terminar los blobs de la copia son identicos a `HEAD`. Tandas de <= 7 min bajo `scripts/heavy-lock.py` (sesiones `i07c-mut-regiones` e `i07c-mut-regiones-c`) con hueco de 75 s. Un MUERTO no se confirmo aislado (la regla del repo basta con el primer fallo). Un timeout de 1.500 s (INCONCLUSO) se repitio una vez sin otras cargas del carril.
- **Caza** (por superviviente, acotada): binario del mutante frente al base (`caza/hunt_build.py` release y `caza/dbg_build.py` debug) en: 1.285 fuentes (`corpus/` y `tests/fixtures/` 217, 152 filas RD, 700 composiciones aleatorias y 216 de cuotas del corpus congelado de S6-COR-2 regenerado con `gen_corpus.py`, mismo SHA-256 de manifiesto `703dfdf8...d0bd`; en la primera tanda ademas 5.685 para M00383 y 500 de las 3.000 de `docs/validation/i07c-s5/s5b1-corpus.py`), 860 dirigidas (`caza/gen_targeted.py`), 600 combinatorias (`caza/gen_tg3.py`) y 192+210 (`caza/gen_tg45.py`); 3 perfiles; `check` (y `run` en la primera tanda).
- **Limites**: carga de la maquina, corpus parcial y 2-3 trabajadores; no se agotaron 20 min por superviviente. "Sin programa" NO afirma equivalencia. La caza de la segunda tanda se hizo sin cerrojo en la fase de comparacion (procesos de cliente, <= 3) y bajo cerrojo en las compilaciones.

## Regresiones nuevas (esperados derivados del ADR, no del binario; fallan con el mutante y pasan sin el, comprobado con `caza/verify_reg.py`)

| Mutante | Commit | Test | Esperado (cita) |
|---|---|---|---|
| S6-COR M00158 (`pattern.span().start` -> `.end` en el `=>` ausente tras la guarda, `parse.rs:3153`) | `7f543ba` | `tests/j14_semantic_boundaries.rs::match_arm_missing_arrow_after_guard_recognizes_prefix_from_the_pattern` | F0-D15 §6.2: `recognized_prefix` del primer token comprometido de la produccion mas interna (el patron, offset 18) al inicio del inesperado (28) en `fn Main(){match 1{1 if true 2=>{},_=>{}}}`; positivo `1 if true=>` aceptado. El mutante da 19. |
| S6-COR M00156 (`guard_context += 2`, `parse.rs:3119`) | `71409cd` | `tests/f0005_l04b.rs::loop_control_after_a_guarded_match_arm_is_not_a_guard_control` | F0-0003 §8.2 y F5-C03/C04: `fn Main(){match 1{1 if true=>{},_=>{}}break;}` -> NX-E1326/break_outside_loop `[38,43)` (`continue` `[38,46)`); dentro de `while` se acepta. El mutante da NX-E1305/guard_control y rechaza un programa valido. |
| S5b.1 M00067 (`loop_depth += 2` en `for`, `types.rs:9527`) | `08237c4` | `tests/f0005_l04b.rs::loop_control_after_a_for_with_a_rejected_iterable_is_outside_the_loop` | F0-0003 §8.2, F5-C03/C04, F0-0002 §4.1: `fn Main(){for x in [1+true]{}break;}` -> NX-E0102 `[22,26)` y NX-E1326 `[29,34)` (`continue` `[29,37)`) en f0-0003 y f0-0005. El mutante pierde NX-E1326. |

Las tres estan en archivos ya registrados en `tests/suite.rs`. `cargo fmt --check` y `cargo clippy --offline --all-targets -- -D warnings` limpios; pasan los 50 tests de `f0005_l04b` y `j14_semantic_boundaries`.

## Supervivientes: deuda con fecha 2026-11-24

Ninguno se declara equivalente salvo el que lleva argumento de invariante; el resto es deuda con motivo concreto, a revisar en la siguiente campana (Codex, tras 2026-11-24 04:43). Un solo programa que distinga a uno lo convierte en defecto con regresion. El motivo de cada uno esta en la ultima columna de las tablas. Argumentos de codigo:

- **S5b.1 M00062** (`StatementWhile(statement, 1)` -> `2`): equivalente por invariante: el segundo campo solo se compara con `0` (`types.rs` 9484-9505; construcciones en 9501 y 10799).
- **S6-COR M00388** (`count_cfg_work(1)` -> `(2)`): `count_cfg_work` solo existe con `cfg(test)` y es no-op fuera de pruebas (`src/ir/mod.rs` ~95 y ~122); solo visible con `measure_cfg_thread_work` (andamiaje).
- **S5b.1 M00348** (`require_copyable(child, span, false)` -> `true`): `child` es una variable recien creada, sin tipo concreto ni `variant_children`; `data_only` solo se consulta con tipo concreto (`types.rs` 4433-4439) y no con `variant_children` (4448-4455).
- **S5b.1 M00168** (`int_dec`, `types.rs:10317`): no termina (dos ejecuciones de 1.500 s con la suite correcta terminando): detectado por no terminacion, sin test que lo nombre.
- **S6-COR M00383** (atajo de `compare_paths`): 5.685 fuentes sin diferencia; sin argumento formal.
- El resto (S6-COR M00029, M00380, M00216, M00164, M00275, M00268; S5b.1 M00388, M00256, M00122, M00036, M00270, M00026, M00081, M00156, M00043, M00380, M00216, M00044, M00143, M00022, M00273, M00170, M00134): indicadores de contexto de `ExprContext`, recorrido de obligaciones, comprobaciones de IR y vectores por id de temporal que ningun test distingue ni programa de la caza exhibe.

## Tiempos y entorno

rustc 1.99.1, `CARGO_BUILD_JOBS=4` (2-3 en la caza), targets `C:/nexo-work/target-mut-regiones` y `...-hunt`. No se paro ningun proceso por nombre de imagen; solo por PID o linea de comando propios (una orden de parada alcanzo en una ocasion un `bash.exe` propio, sin efecto en el trabajo). Copias y targets borrados al terminar.

## Archivos

`docs/validation/i07c-mut-regiones/`: `mut_population.py`, `population-{s6cor,s5b1}.json`, `population.sha256`, `run_mut.py`, `results-{s6cor,s5b1}.jsonl` (una fila por mutante) y `results-s5b1-inconclusos.jsonl` (ejecuciones con timeout repetidas), `hunt-{s6cor,s5b1}.json` (notas de caza), `gen_tables.py`, `caza/`.

## Tablas por mutante

### S6-COR (`1010938..2bc5c8d`; `parse.rs`, `effects.rs`, `names.rs`, `types.rs`)

Población 400 (enumeración completa 1313), ejecutados 279: INCONCLUSO 3, MUERTO 181, NO COMPILA 48, SUPERVIVIENTE 47

| id | archivo:línea | operador | SHA ini | SHA mut | veredicto | test que lo mata / nota |
|---|---|---|---|---|---|---|
| M00300 | src/ir/effects.rs:883 | or | 7557010d | 33ebd618 | NO COMPILA |  |
| M00205 | src/frontend/parse.rs:3434 | true | 1793866d | 0c27273a | MUERTO | debug: frontend::migrated_parser_final::b_first_consistency_across_nested_grammar_positions |
| M00066 | src/check/types.rs:12356 | drop_return | 54c9dfed | cd4e8924 | MUERTO | debug: ir::i07a_tests::direct_lowering_discards_divergent_composite_index_operands |
| M00229 | src/frontend/parse.rs:3673 | int_dec | 1793866d | c2453641 | MUERTO | debug: frontend::rows::literal_branch_and_contract_rows_have_complete_parser_diagnostics |
| M00179 | src/frontend/parse.rs:3290 | eq | 1793866d | 087ade34 | MUERTO | debug: check::tests::adr_gap_grouped_absent_target_keeps_call_target_use_kind |
| M00121 | src/frontend/parse.rs:2768 | negate_if | 1793866d | 279f4a9a | MUERTO | debug: check::tests::divergent_binding_read_preserves_unresolved_inference |
| M00132 | src/frontend/parse.rs:2817 | drop_state | 1793866d | 8813cbe8 | MUERTO | debug: check::tests::conditions_require_bool_and_assignment_keeps_written_type_origin |
| M00395 | src/ir/effects.rs:4263 | int_inc | 7557010d | 93658c5d | MUERTO | debug: ir::effects::tests::call_restrictions_use_transitive_effect_summaries |
| M00351 | src/ir/effects.rs:1924 | int_inc | 7557010d | ee63b0cf | MUERTO | debug: ir::effects::tests::unknown_effects_fail_each_written_restriction_in_code_order |
| M00029 | src/check/types.rs:11978 | start_end | 54c9dfed | e7459869 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285 fuentes release x3 perfiles x check/run + 860 + 168 + 600 dirigidas) |
| M00081 | src/check/types.rs:12542 | swap_args | 54c9dfed | 8ef00e97 | NO COMPILA |  |
| M00388 | src/ir/effects.rs:4223 | int_inc | 7557010d | 3753b07b | SUPERVIVIENTE | SIN PROGRAMA: `count_cfg_work` es `cfg(test)` y no-op fuera de pruebas (`src/ir/mod.rs` 95 y 122); solo observable con `measure_cfg_thread_work` (andamiaje de pruebas); 860 dirigidas sin diferencia |
| M00067 | src/check/types.rs:12406 | swap_args | 54c9dfed | 79d84e7d | NO COMPILA |  |
| M00270 | src/ir/effects.rs:150 | swap_args | 7557010d | 8cfde70c | NO COMPILA |  |
| M00360 | src/ir/effects.rs:2033 | int_inc | 7557010d | ea6adc97 | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00383 | src/ir/effects.rs:4126 | drop_return | 7557010d | f1f655ca | SUPERVIVIENTE | SIN PROGRAMA tras la caza (5.685 fuentes release x3 x check/run + 860 dirigidas); atajo de comparacion de `compare_paths` (comentario del codigo: evita recorrer el sufijo); no se afirma equivalencia |
| M00261 | src/frontend/parse.rs:3931 | and | 1793866d | c6dd9c3b | MUERTO | debug: frontend::parse::b_tests::hidden_f5_range_level_has_adr_ast_shape_across_both_sides |
| M00158 | src/frontend/parse.rs:3153 | start_end | 1793866d | ca9caf20 | SUPERVIVIENTE | DEFECTO DE COBERTURA con programa `fn Main(){match 1{1 if true 2=>{},_=>{}}}` (recognized_prefix [18,28) vs [19,28)); regresion 7f543ba |
| M00026 | src/check/types.rs:11974 | and | 54c9dfed | 63abcd26 | NO COMPILA |  |
| M00280 | src/ir/effects.rs:510 | and | 7557010d | 03033ece | NO COMPILA |  |
| M00036 | src/check/types.rs:12005 | end_start | 54c9dfed | 361a638f | MUERTO | debug: nexo::suite$j11_generator::generator_question_mark_and_shadowed_iterator_limit_forms |
| M00247 | src/frontend/parse.rs:3859 | continue | 1793866d | 4c272b58 | MUERTO | debug: ir::verify::tests::j16_forged_generator_yield_item_is_rejected |
| M00006 | src/check/names.rs:1193 | or | 3363b499 | 42e682e1 | NO COMPILA |  |
| M00296 | src/ir/effects.rs:850 | negate_if | 7557010d | a788de15 | MUERTO | debug: ir::effects::tests::published_effect_fixture_fields_match_the_literal_contract |
| M00348 | src/ir/effects.rs:1881 | int_inc | 7557010d | 32a21579 | MUERTO | debug: nexo::suite$i07c_group_primary::effect_diagnostics_report_the_target_name_of_a_grouped_call |
| M00202 | src/frontend/parse.rs:3421 | end_start | 1793866d | fe16bd19 | MUERTO | debug: check::types::quota_tests::partial_type_table_preserves_never_and_taint_states |
| M00024 | src/check/types.rs:11973 | and | 54c9dfed | 29a26641 | NO COMPILA |  |
| M00358 | src/ir/effects.rs:2007 | or | 7557010d | 726e5330 | NO COMPILA |  |
| M00378 | src/ir/effects.rs:2153 | eq | 7557010d | 7b71b0ce | MUERTO | debug: frontend::tests::multi_token_construct_comment_gaps_preserve_execution |
| M00122 | src/frontend/parse.rs:2772 | swap_args | 1793866d | e2284389 | MUERTO | debug: frontend::migrated_parser_contracts::b_branch_value_negatives_keep_codes_spans_and_related_locations |
| M00008 | src/check/names.rs:1208 | eq | 3363b499 | a9b59d8d | MUERTO | debug: nexo::suite$i07c_group_primary::call_binding_diagnostics_report_the_inner_target_and_argument |
| M00256 | src/frontend/parse.rs:3883 | and | 1793866d | 061833a7 | MUERTO | debug: check::types::quota_tests::boundary_binary_deferred_rhs_keeps_owner_until_result |
| M00210 | src/frontend/parse.rs:3447 | drop_return | 1793866d | d7c209c2 | NO COMPILA |  |
| M00315 | src/ir/effects.rs:1157 | swap_args | 7557010d | bac38b06 | NO COMPILA |  |
| M00062 | src/check/types.rs:12337 | drop_return | 54c9dfed | 0551e970 | NO COMPILA |  |
| M00082 | src/check/types.rs:12560 | swap_args | 54c9dfed | 499ba7a1 | NO COMPILA |  |
| M00326 | src/ir/effects.rs:1538 | break | 7557010d | 05e51077 | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00025 | src/check/types.rs:11973 | start_end | 54c9dfed | 99c4bc39 | MUERTO | debug: nexo::suite$i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin |
| M00156 | src/frontend/parse.rs:3119 | int_inc | 1793866d | e752534a | SUPERVIVIENTE | DEFECTO DE COBERTURA con programa `fn Main(){match 1{1 if true=>{},_=>{}}break;}` (NX-E1326 vs NX-E1305/guard_control; `while true{...break;}` rechazado); regresion 71409cd |
| M00058 | src/check/types.rs:12304 | swap_args | 54c9dfed | 9b3ddaa8 | NO COMPILA |  |
| M00197 | src/frontend/parse.rs:3386 | start_end | 1793866d | ff998e74 | MUERTO | debug: frontend::migrated_parser_contracts::b_prefix_place_reports_first_non_place_token |
| M00176 | src/frontend/parse.rs:3288 | drop_state | 1793866d | f51ab921 | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00357 | src/ir/effects.rs:2005 | end_start | 7557010d | e4e8dc74 | MUERTO | debug: ir::effects::tests::unknown_effects_fail_each_written_restriction_in_code_order |
| M00043 | src/check/types.rs:12085 | end_start | 54c9dfed | b6a99555 | MUERTO | debug: nexo::suite$f0007::try_rejects_a_concrete_error_component_different_from_the_function |
| M00380 | src/ir/effects.rs:2161 | drop_return | 7557010d | b2e765e5 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); el `Err` solo se produce ante un IR de lowering inconsistente (verificador de salidas), sin fuente conocida que lo provoque |
| M00068 | src/check/types.rs:12422 | swap_args | 54c9dfed | 37c5253b | NO COMPILA |  |
| M00118 | src/frontend/parse.rs:2761 | add | 1793866d | 983fbfeb | MUERTO | debug: check::tests::divergent_binding_read_preserves_unresolved_inference |
| M00216 | src/frontend/parse.rs:3471 | drop_assignment | 1793866d | 95f79a1e | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); con `[MockOpen(1)]` la operacion sigue siendo types.check con y sin la asignacion |
| M00164 | src/frontend/parse.rs:3199 | start_end | 1793866d | 0286c421 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); NX-E1304 usa `keyword_span`, no `Match.span` |
| M00267 | src/frontend/parse.rs:3969 | drop_assignment | 1793866d | d6f50d25 | MUERTO | debug: check::tests::adr_gap_function_name_is_not_an_assignable_place |
| M00044 | src/check/types.rs:12112 | drop_state | 54c9dfed | 5b8f6803 | MUERTO | debug: nexo::suite$f0007::try_constrains_result_error_and_never_does_not_constrain_payload |
| M00298 | src/ir/effects.rs:861 | int_inc | 7557010d | 4ba9ba7c | MUERTO | debug: nexo::suite$cli::cli_mock_write_requires_io_effect |
| M00048 | src/check/types.rs:12213 | and | 54c9dfed | fb88cf54 | NO COMPILA |  |
| M00143 | src/frontend/parse.rs:2959 | end_start | 1793866d | f1d25a39 | MUERTO | debug: ir::effects::tests::ownership_liveness_propagates_across_cfg_predecessors |
| M00168 | src/frontend/parse.rs:3228 | increment | 1793866d | 10b9e287 | MUERTO | debug: check::tests::annotation_closing_angles_exclude_assignment_equals |
| M00213 | src/frontend/parse.rs:3465 | swap_args | 1793866d | 26fd339a | MUERTO | debug: check::tests::array_element_context_and_empty_array_inference_are_independent |
| M00283 | src/ir/effects.rs:534 | int_inc | 7557010d | 9a9427db | MUERTO | debug: ir::effects::tests::distinct_constraints_emit_once_each_and_print_does_not_imply_alloc |
| M00338 | src/ir/effects.rs:1730 | end_start | 7557010d | 76b6dd70 | MUERTO | debug: nexo::suite$j09_effects::written_closure_operation_bounds_use_capture_witnesses |
| M00102 | src/frontend/parse.rs:2627 | start_end | 1793866d | a23a3807 | NO COMPILA |  |
| M00019 | src/check/types.rs:11921 | negate_if | 54c9dfed | 566124d2 | MUERTO | debug: ir::lower::tests::f5_neg_reaches_lowering_boundary |
| M00282 | src/ir/effects.rs:527 | and | 7557010d | 2fd18738 | MUERTO | debug: ir::effects::tests::distinct_constraints_emit_once_each_and_print_does_not_imply_alloc |
| M00022 | src/check/types.rs:11935 | drop_return | 54c9dfed | d0afb1af | MUERTO | debug: nexo::suite$f0005_b1b::f5_declared_constants_remain_pending_at_every_use_site |
| M00201 | src/frontend/parse.rs:3419 | swap_args | 1793866d | ebdcab59 | NO COMPILA |  |
| M00107 | src/frontend/parse.rs:2684 | negate_if | 1793866d | 365e0a8e | MUERTO | debug: check::types::j16_probe_types::parent |
| M00273 | src/ir/effects.rs:170 | swap_args | 7557010d | 1c312ea9 | NO COMPILA |  |
| M00170 | src/frontend/parse.rs:3228 | int_inc | 1793866d | ebfe11b9 | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00015 | src/check/types.rs:8305 | false | 54c9dfed | 61f840aa | MUERTO | debug: nexo::suite$j13_iteration::ex04_error_and_early_exit_paths_close_the_generator |
| M00275 | src/ir/effects.rs:488 | int_inc | 7557010d | 5311d168 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); solo agranda en 1 los vectores por id de temporal |
| M00268 | src/ir/effects.rs:142 | min_max | 7557010d | ebbd7eb4 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); requiere dos llamadas escritas con el mismo inicio de sitio y distinto inicio de llamada en un empate de diamante: no se construyo |
| M00134 | src/frontend/parse.rs:2818 | eq | 1793866d | 7375a9c2 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00189 | src/frontend/parse.rs:3357 | negate_if | 1793866d | 67583e1f | MUERTO | debug: check::tests::adr_gap_inout_to_ordinary_parameter_uses_place_primary |
| M00094 | src/frontend/parse.rs:2591 | negate_if | 1793866d | 5338de12 | MUERTO | debug: check::names::tests::function_scope_restores_preexisting_resolver_state |
| M00317 | src/ir/effects.rs:1182 | drop_state | 7557010d | c96cd3f7 | MUERTO | debug: ir::effects::witness_differential_tests::grouped_non_name_targets_report_their_head_and_order_by_the_call |
| M00290 | src/ir/effects.rs:629 | int_inc | 7557010d | a2320e44 | MUERTO | debug: ir::i10b_tests::drop_binding_machine_charge_is_observable_on_a_verified_ir_route |
| M00316 | src/ir/effects.rs:1170 | drop_state | 7557010d | c93997a8 | MUERTO | debug: nexo::suite$i07c_group_primary::effect_diagnostics_report_the_target_name_of_a_grouped_call |
| M00369 | src/ir/effects.rs:2098 | negate_if | 7557010d | 94a38dc8 | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00353 | src/ir/effects.rs:1975 | drop_assignment | 7557010d | 87de17e7 | MUERTO | debug: ir::effects::tests::unknown_effects_fail_each_written_restriction_in_code_order |
| M00244 | src/frontend/parse.rs:3845 | or | 1793866d | c983bac7 | MUERTO | debug: nexo::suite$j11_generator::generator_yield_next_and_return_rules_have_paired_sources |
| M00262 | src/frontend/parse.rs:3932 | and | 1793866d | 46671d1e | MUERTO | debug: frontend::parse::b_tests::hidden_f5_range_level_has_adr_ast_shape_across_both_sides |
| M00377 | src/ir/effects.rs:2151 | drop_return | 7557010d | b44bc496 | MUERTO | debug: ir::effects::tests::return_exit_may_start_at_the_return_terminator |
| M00228 | src/frontend/parse.rs:3643 | start_end | 1793866d | 1d704c90 | MUERTO | debug: ir::i07a_tests::directed_branch_error_and_divergence_fuzz_has_no_internal_failure |
| M00381 | src/ir/effects.rs:4116 | negate_if | 7557010d | 5b7bedab | MUERTO | debug: ir::effects::tests::canonical_effect_witness_prefers_shorter_path_and_ascending_offsets |
| M00222 | src/frontend/parse.rs:3544 | end_start | 1793866d | 4910abce | MUERTO | debug: frontend::migrated_parser_contracts::b_terminator_repair_uses_expression_end_and_exact_inserted_text |
| M00366 | src/ir/effects.rs:2067 | int_inc | 7557010d | 5c9f2359 | MUERTO | debug: ir::verify::tests::j16_forged_generator_resume_item_is_rejected |
| M00007 | src/check/names.rs:1208 | and | 3363b499 | f01fe766 | MUERTO | debug: nexo::suite$i07c_group_primary::call_binding_diagnostics_report_the_inner_target_and_argument |
| M00115 | src/frontend/parse.rs:2752 | swap_args | 1793866d | b7a04f83 | MUERTO | debug: check::tests::adr_gap_explicit_empty_return_uses_keyword_primary |
| M00386 | src/ir/effects.rs:4203 | swap_args | 7557010d | e12cea8d | NO COMPILA |  |
| M00161 | src/frontend/parse.rs:3182 | drop_state | 1793866d | 98e957ae | MUERTO | debug: check::names::tests::constructor_pattern_binds_nested_name_before_arm_body |
| M00367 | src/ir/effects.rs:2083 | and | 7557010d | 226a59af | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00054 | src/check/types.rs:12288 | swap_args | 54c9dfed | 3e4997f2 | NO COMPILA |  |
| M00139 | src/frontend/parse.rs:2846 | drop_state | 1793866d | bd19a0af | MUERTO | debug: check::tests::absent_value_and_call_target_use_distinct_kinds |
| M00042 | src/check/types.rs:12070 | int_inc | 54c9dfed | 0f1ccd5e | MUERTO | debug: nexo::suite$i08::published_result_type_negatives_and_repairs |
| M00387 | src/ir/effects.rs:4221 | int_inc | 7557010d | dd3eccff | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00028 | src/check/types.rs:11978 | all_any | 54c9dfed | deb95f2c | MUERTO | debug: nexo::suite$i07c_mutation_regressions::anchored_first_branch_keeps_independent_pending_join |
| M00027 | src/check/types.rs:11974 | lt | 54c9dfed | 2dd8168c | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00195 | src/frontend/parse.rs:3372 | negate_if | 1793866d | 8d693152 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00277 | src/ir/effects.rs:504 | and | 7557010d | cf21d82f | NO COMPILA |  |
| M00230 | src/frontend/parse.rs:3675 | negate_if | 1793866d | d90f13bc | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00325 | src/ir/effects.rs:1536 | all_any | 7557010d | c8e02ed6 | MUERTO | debug: ir::effects::tests::recursive_call_components_converge_and_keep_known_leaf_witnesses |
| M00076 | src/check/types.rs:12475 | eq | 54c9dfed | 9de83b79 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00049 | src/check/types.rs:12214 | and | 54c9dfed | c4fbef6f | NO COMPILA |  |
| M00304 | src/ir/effects.rs:929 | continue | 7557010d | 4f70fe77 | MUERTO | debug: ir::effects::tests::borrowed_user_call_skips_borrowed_bindings_but_visits_later_temporaries |
| M00269 | src/ir/effects.rs:143 | start_end | 7557010d | ffddbeb1 | MUERTO | debug: ir::effects::witness_differential_tests::grouped_non_name_targets_report_their_head_and_order_by_the_call |
| M00364 | src/ir/effects.rs:2058 | or | 7557010d | 275dda02 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00253 | src/frontend/parse.rs:3877 | swap_args | 1793866d | 7719d37f | NO COMPILA |  |
| M00370 | src/ir/effects.rs:2099 | drop_state | 7557010d | 57f68be7 | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00331 | src/ir/effects.rs:1607 | start_end | 7557010d | b65ef858 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00376 | src/ir/effects.rs:2147 | false | 7557010d | 55bef97c | MUERTO | debug: ir::effects::tests::verifier_rejects_a_return_exit_on_a_non_return_terminator |
| M00160 | src/frontend/parse.rs:3173 | end_start | 1793866d | db11867b | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00063 | src/check/types.rs:12340 | negate_if | 54c9dfed | e817811e | MUERTO | debug: check::tests::conditional_selector_constraint_blames_the_later_conflicting_value |
| M00031 | src/check/types.rs:11978 | start_end | 54c9dfed | 9420049c | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00297 | src/ir/effects.rs:861 | ne | 7557010d | 5451e26c | MUERTO | debug: ir::effects::tests::unknown_effects_fail_each_written_restriction_in_code_order |
| M00013 | src/check/types.rs:8270 | start_end | 54c9dfed | 844284f9 | MUERTO | debug: nexo::suite$f0005_b1b::pending_f5_destinations_and_const_have_exact_public_diagnostics |
| M00303 | src/ir/effects.rs:926 | ne | 7557010d | ef1c6568 | MUERTO | debug: ir::effects::tests::borrowed_user_call_counts_the_argument_temporary_in_the_caller |
| M00209 | src/frontend/parse.rs:3444 | drop_assignment | 1793866d | 4e82a5c8 | MUERTO | debug: frontend::tests::adr_literal_f2_table_rows_reach_the_published_frontend_boundary |
| M00274 | src/ir/effects.rs:486 | add | 7557010d | 72ae527b | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00324 | src/ir/effects.rs:1534 | drop_assignment | 7557010d | 2f6469ab | MUERTO | debug: check::tests::nested_pending_nominal_result_has_executable_binding_type |
| M00194 | src/frontend/parse.rs:3362 | and | 1793866d | 4ec8ccdd | NO COMPILA |  |
| M00334 | src/ir/effects.rs:1661 | int_inc | 7557010d | 14cc591a | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00059 | src/check/types.rs:12310 | negate_if | 54c9dfed | ec9a58b7 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00075 | src/check/types.rs:12472 | end_start | 54c9dfed | 87738f01 | MUERTO | debug: check::tests::absent_value_and_call_target_use_distinct_kinds |
| M00337 | src/ir/effects.rs:1683 | int_inc | 7557010d | d5cd2446 | MUERTO | debug: nexo::suite$f0005_b1b::f5_effect_contracts_publish_the_profile_model |
| M00165 | src/frontend/parse.rs:3224 | and | 1793866d | da9a1bba | MUERTO | debug: check::names::tests::constructor_pattern_binds_nested_name_before_arm_body |
| M00365 | src/ir/effects.rs:2066 | int_inc | 7557010d | 46104729 | MUERTO | debug: ir::verify::tests::j16_forged_generator_resume_item_is_rejected |
| M00159 | src/frontend/parse.rs:3173 | swap_args | 1793866d | 6573b409 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00211 | src/frontend/parse.rs:3450 | continue | 1793866d | 44692b8b | MUERTO | debug: check::tests::adr_gap_array_element_mismatch_uses_element_primary_and_whole_annotation_context |
| M00053 | src/check/types.rs:12279 | true | 54c9dfed | 18668467 | MUERTO | debug: nexo::suite$i07c_normative_survivors::deferred_projection_and_constructor_programs_finish_with_causal_results |
| M00041 | src/check/types.rs:12070 | int_dec | 54c9dfed | 9013548e | MUERTO | debug: nexo::suite$i08::published_result_type_negatives_and_repairs |
| M00065 | src/check/types.rs:12344 | drop_return | 54c9dfed | 3e35a6aa | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00343 | src/ir/effects.rs:1811 | continue | 7557010d | c6e90896 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00080 | src/check/types.rs:12540 | negate_if | 54c9dfed | 16c16781 | MUERTO | debug: check::types::quota_tests::nested_divergent_branch_contributes_no_normal_type_under_context |
| M00190 | src/frontend/parse.rs:3358 | drop_return | 1793866d | 507d4666 | MUERTO | debug: frontend::migrated_parser_contracts::b_repeated_and_grouped_modes_classify_after_the_whole_postfix |
| M00276 | src/ir/effects.rs:498 | and | 7557010d | cbe60deb | NO COMPILA |  |
| M00097 | src/frontend/parse.rs:2592 | decrement | 1793866d | f412469f | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00384 | src/ir/effects.rs:4138 | or | 7557010d | 1fd6ce01 | NO COMPILA |  |
| M00069 | src/check/types.rs:12423 | drop_return | 54c9dfed | 721685d5 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00096 | src/frontend/parse.rs:2591 | int_inc | 1793866d | c1316997 | MUERTO | debug: frontend::migrated_parser_final::b_structural_budgets_have_literal_boundaries_and_diagnostic_fields |
| M00157 | src/frontend/parse.rs:3151 | int_dec | 1793866d | e778b1ee | MUERTO | debug: nexo::suite$f0005_l04b::loop_control_after_a_guarded_match_arm_is_not_a_guard_control |
| M00021 | src/check/types.rs:11934 | eq | 54c9dfed | bde92f8b | MUERTO | debug: ir::lower::tests::f5_source_logical_cfg_skips_only_untaken_rhs |
| M00109 | src/frontend/parse.rs:2711 | negate_if | 1793866d | 323b73dd | MUERTO | debug: check::names::tests::constructor_pattern_binds_nested_name_before_arm_body |
| M00266 | src/frontend/parse.rs:3967 | decrement | 1793866d | 6117a30d | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00110 | src/frontend/parse.rs:2723 | is_some | 1793866d | cd7cb710 | MUERTO | debug: frontend::migrated_parser_final::b_first_sets_follow_the_committed_production |
| M00003 | src/check/names.rs:1071 | and | 3363b499 | f9109bdd | NO COMPILA |  |
| M00234 | src/frontend/parse.rs:3739 | drop_assignment | 1793866d | 9faafca2 | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00128 | src/frontend/parse.rs:2795 | int_inc | 1793866d | 342b32e9 | MUERTO | debug: frontend::migrated_parser_final::b_related_locations_use_first_duplicate_and_full_crlf_delimiter |
| M00174 | src/frontend/parse.rs:3277 | start_end | 1793866d | c7915e38 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00193 | src/frontend/parse.rs:3358 | int_inc | 1793866d | 0afe0738 | MUERTO | debug: frontend::migrated_parser_contracts::b_repeated_and_grouped_modes_classify_after_the_whole_postfix |
| M00249 | src/frontend/parse.rs:3861 | swap_args | 1793866d | 8c392bcd | NO COMPILA |  |
| M00090 | src/frontend/parse.rs:2510 | swap_args | 1793866d | ab4d4f5b | NO COMPILA |  |
| M00329 | src/ir/effects.rs:1575 | continue | 7557010d | 1fe93035 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00177 | src/frontend/parse.rs:3289 | true | 1793866d | dd39bce5 | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00400 | src/ir/effects.rs:4289 | drop_assignment | 7557010d | 5e0026f1 | MUERTO | debug: ir::effects::tests::transitive_witness_preserves_call_chain_and_diagnostic_operation |
| M00171 | src/frontend/parse.rs:3230 | negate_if | 1793866d | b24bc120 | MUERTO | debug: check::names::tests::constructor_pattern_binds_nested_name_before_arm_body |
| M00120 | src/frontend/parse.rs:2763 | is_none | 1793866d | 04b363c7 | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00285 | src/ir/effects.rs:551 | int_inc | 7557010d | 826bee72 | MUERTO | debug: ir::effects::tests::no_alloc_counts_array_copies_from_index_and_result_bindings |
| M00154 | src/frontend/parse.rs:3105 | or | 1793866d | 3ffa735d | MUERTO | debug: check::names::tests::j16_names_families_finish_in_256_kib_worker |
| M00087 | src/check/types.rs:12592 | swap_args | 54c9dfed | 329b7e95 | NO COMPILA |  |
| M00306 | src/ir/effects.rs:939 | eq | 7557010d | 62ab7d10 | MUERTO | debug: ir::effects::tests::borrowed_user_call_counts_the_argument_temporary_in_the_caller |
| M00079 | src/check/types.rs:12531 | is_some | 54c9dfed | 3f244ebe | MUERTO | debug: nexo::suite$i07c_mutation_regressions::anchored_first_branch_keeps_independent_pending_join |
| M00335 | src/ir/effects.rs:1679 | int_inc | 7557010d | bf9d8e5f | MUERTO | debug: nexo::suite$f0005_b1b::f5_effect_contracts_publish_the_profile_model |
| M00363 | src/ir/effects.rs:2046 | int_inc | 7557010d | cf9f0265 | MUERTO | debug: frontend::tests::multi_token_construct_comment_gaps_preserve_execution |
| M00149 | src/frontend/parse.rs:3040 | drop_assignment | 1793866d | 3ea94c41 | MUERTO | debug: check::tests::if_else_scope_does_not_erase_outer_binding |
| M00319 | src/ir/effects.rs:1186 | drop_state | 7557010d | 2789fea6 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00340 | src/ir/effects.rs:1757 | swap_args | 7557010d | 1ba97e45 | NO COMPILA |  |
| M00126 | src/frontend/parse.rs:2780 | add | 1793866d | 99ef754b | MUERTO | debug: frontend::migrated_parser_contracts::b_branch_value_negatives_keep_codes_spans_and_related_locations |
| M00169 | src/frontend/parse.rs:3228 | int_dec | 1793866d | 3bca4a38 | MUERTO | debug: check::names::tests::constructor_pattern_binds_nested_name_before_arm_body |
| M00033 | src/check/types.rs:11983 | swap_args | 54c9dfed | b7576a3b | NO COMPILA |  |
| M00235 | src/frontend/parse.rs:3762 | start_end | 1793866d | d9c5fe09 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |


### S5b.1 (`4acee20..1010938`; `types.rs`)

Población 400 (enumeración completa 1048), ejecutados 273: INCONCLUSO 10, MUERTO 138, NO COMPILA 55, SUPERVIVIENTE 70

| id | archivo:línea | operador | SHA ini | SHA mut | veredicto | test que lo mata / nota |
|---|---|---|---|---|---|---|
| M00300 | src/check/types.rs:11253 | true | 54c9dfed | ec366662 | MUERTO | debug: check::tests::mock_write_i64_rejects_non_i64_argument |
| M00205 | src/check/types.rs:10699 | drop_assignment | 54c9dfed | da20199b | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00066 | src/check/types.rs:9525 | drop_return | 54c9dfed | 35f4e88e | MUERTO | debug: nexo::suite$f0005_b1b::pending_f5_destinations_and_const_have_exact_public_diagnostics |
| M00229 | src/check/types.rs:10815 | continue | 54c9dfed | 9e31dbaa | NO COMPILA |  |
| M00179 | src/check/types.rs:10433 | drop_assignment | 54c9dfed | e199171a | MUERTO | debug: nexo::suite$i07c_s4_context_regressions::contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli |
| M00121 | src/check/types.rs:9866 | swap_args | 54c9dfed | 86cfbd89 | NO COMPILA |  |
| M00132 | src/check/types.rs:9966 | gt | 54c9dfed | 92966cac | MUERTO | debug: check::tests::all_preparsed_names_are_resolved_by_lexical_position |
| M00395 | src/check/types.rs:12544 | is_some | 54c9dfed | 8f59aab6 | MUERTO | debug: ir::i07c_tests::plus_distributed_left_failure_suppresses_second_domain_error |
| M00351 | src/check/types.rs:12153 | drop_return | 54c9dfed | 8ee8ceda | NO COMPILA |  |
| M00029 | src/check/types.rs:9168 | int_dec | 54c9dfed | eaa9afb0 | MUERTO | debug: check::types::j16_probe_types::parent |
| M00081 | src/check/types.rs:9556 | int_dec | 54c9dfed | 06883606 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release, 860+168+600 dirigidas) |
| M00388 | src/check/types.rs:12476 | drop_assignment | 54c9dfed | c501b589 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00067 | src/check/types.rs:9527 | int_inc | 54c9dfed | c4fdd51a | SUPERVIVIENTE | DEFECTO DE COBERTURA con programa `fn Main(){for x in [1+true]{}break;}` (falta NX-E1326 en `break`); regresion 08237c4 |
| M00270 | src/check/types.rs:11037 | false | 54c9dfed | ade153c1 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00360 | src/check/types.rs:12264 | swap_args | 54c9dfed | 5fd1fb15 | NO COMPILA |  |
| M00383 | src/check/types.rs:12451 | drop_record | 54c9dfed | ea1efee3 | MUERTO | debug: check::tests::selector_equality_error_survives_non_array_base |
| M00261 | src/check/types.rs:10979 | swap_args | 54c9dfed | 75972cbb | NO COMPILA |  |
| M00158 | src/check/types.rs:10229 | drop_assignment | 54c9dfed | 3473735d | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00026 | src/check/types.rs:9150 | false | 54c9dfed | 46fee754 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00280 | src/check/types.rs:11082 | int_inc | 54c9dfed | 41db72da | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00036 | src/check/types.rs:9297 | false | 54c9dfed | 5409496f | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00247 | src/check/types.rs:10888 | int_dec | 54c9dfed | c701d7fc | MUERTO | debug: check::types::j16_probe_types::parent |
| M00006 | src/check/types.rs:9004 | drop_assignment | 54c9dfed | c7be8143 | MUERTO | debug: nexo::suite$i07c_mutation_regressions::numeric_visit_owner_limits_deferred_joins_to_one_domain_error |
| M00296 | src/check/types.rs:11216 | int_inc | 54c9dfed | 9526649c | MUERTO | debug: check::types::quota_tests::tainted_array_stops_after_the_first_denied_charge |
| M00348 | src/check/types.rs:12144 | false | 54c9dfed | 92252067 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes, 860+600 dirigidas); argumento de invariante: `child` nace de `new_variable` en la misma rama (types.rs ~12135), `concrete` es None y `variant_children` None, y `data_only` solo se consulta en esos dos casos (`require_copyable`, types.rs 4423-4460) |
| M00202 | src/check/types.rs:10670 | continue | 54c9dfed | c1f112a8 | NO COMPILA |  |
| M00024 | src/check/types.rs:9146 | true | 54c9dfed | 494f13a6 | MUERTO | debug: nexo::suite$j05_match_fuzz::nested_guarded_match_resource_and_copy_sweep |
| M00358 | src/check/types.rs:12214 | and | 54c9dfed | c4fbef6f | NO COMPILA |  |
| M00378 | src/check/types.rs:12410 | drop_return | 54c9dfed | 0133400f | MUERTO | debug: check::tests::conditional_selector_constraint_blames_the_later_conflicting_value |
| M00122 | src/check/types.rs:9869 | any_all | 54c9dfed | 8cc64ebf | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00008 | src/check/types.rs:9029 | and | 54c9dfed | 29665909 | MUERTO | debug: ir::lower::tests::f5_array_for_borrows_source_without_defensive_copy |
| M00256 | src/check/types.rs:10921 | drop_assignment | 54c9dfed | f78d7a59 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (1.285+500 fuentes release x3 x check/run, 860 dirigidas release y 600 dirigidas debug); no se afirma equivalencia |
| M00210 | src/check/types.rs:10739 | drop_assignment | 54c9dfed | b48cd7e4 | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00315 | src/check/types.rs:11904 | negate_if | 54c9dfed | 48642499 | MUERTO | debug: check::tests::absent_value_and_call_target_use_distinct_kinds |
| M00062 | src/check/types.rs:9501 | int_inc | 54c9dfed | 05053ed9 | SUPERVIVIENTE | EQUIVALENTE por argumento de invariante: `Frame::StatementWhile(_, stage)` solo se compara con `stage == 0` (types.rs 9484-9505; construcciones en 9501 y 10799), asi que 1 y 2 son indistinguibles |
| M00082 | src/check/types.rs:9556 | int_inc | 54c9dfed | 1237fc5a | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00326 | src/check/types.rs:11973 | and | 54c9dfed | 29a26641 | NO COMPILA |  |
| M00025 | src/check/types.rs:9149 | is_some | 54c9dfed | 542938b8 | MUERTO | debug: nexo::suite$j05_match_guards::guard_requires_bool_before_ownership_and_effect_checks |
| M00156 | src/check/types.rs:10198 | false | 54c9dfed | 14ffe28a | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00058 | src/check/types.rs:9489 | swap_args | 54c9dfed | 62f8cfbf | NO COMPILA |  |
| M00197 | src/check/types.rs:10623 | swap_args | 54c9dfed | d5f8638a | NO COMPILA |  |
| M00176 | src/check/types.rs:10416 | true | 54c9dfed | dd518cda | MUERTO | debug: nexo::suite$i07c_causal_obligations::independent_operands_keep_their_causal_sites |
| M00357 | src/check/types.rs:12213 | and | 54c9dfed | fb88cf54 | NO COMPILA |  |
| M00043 | src/check/types.rs:9430 | drop_assignment | 54c9dfed | 69690aa6 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00380 | src/check/types.rs:12433 | drop_return | 54c9dfed | 01af1e3e | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00068 | src/check/types.rs:9528 | int_dec | 54c9dfed | 328e65a3 | MUERTO | debug: check::types::j16_probe_types::parent |
| M00118 | src/check/types.rs:9856 | or | 54c9dfed | d66dcbbc | MUERTO | debug: check::tests::branch_restores_dependency_taint_after_invalid_return_expression |
| M00216 | src/check/types.rs:10766 | drop_state | 54c9dfed | 9c3f79e1 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00164 | src/check/types.rs:10308 | drop_assignment | 54c9dfed | c637607b | MUERTO | debug: check::types::quota_tests::boundary_closure_call_restores_parent_fact |
| M00267 | src/check/types.rs:11018 | drop_state | 54c9dfed | f180c210 | MUERTO | debug: ir::lower::tests::f5_neg_reaches_lowering_boundary |
| M00044 | src/check/types.rs:9430 | false | 54c9dfed | 217b0caa | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00298 | src/check/types.rs:11244 | start_end | 54c9dfed | ab7e05d1 | MUERTO | debug: check::tests::adr_gap_primitive_domain_primary_is_first_offending_argument |
| M00048 | src/check/types.rs:9456 | drop_assignment | 54c9dfed | 1ea87360 | NO COMPILA |  |
| M00143 | src/check/types.rs:10104 | drop_assignment | 54c9dfed | 56cb2267 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00168 | src/check/types.rs:10317 | int_dec | 54c9dfed | ac8e619a | INCONCLUSO | NO TERMINA: dos ejecuciones en debug sin otras cargas del propio carril, 1.500 s cada una (timeout), la suite correcta termina; detectado por no terminacion, sin test que lo nombre |
| M00213 | src/check/types.rs:10756 | or | 54c9dfed | dadc5a6c | MUERTO | debug: check::types::j16_probe_types::parent |
| M00283 | src/check/types.rs:11089 | int_inc | 54c9dfed | bf4b4285 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00338 | src/check/types.rs:12070 | int_dec | 54c9dfed | 9013548e | MUERTO | debug: nexo::suite$i08::published_result_type_negatives_and_repairs |
| M00102 | src/check/types.rs:9736 | false | 54c9dfed | 15f07a64 | MUERTO | debug: nexo::suite$i07c_mutation_regressions::unresolved_constructors_close_from_their_own_obligations |
| M00019 | src/check/types.rs:9107 | eq | 54c9dfed | dc7e7fa2 | MUERTO | debug: check::types::j16_probe_types::parent |
| M00282 | src/check/types.rs:11089 | drop_assignment | 54c9dfed | 9fb11378 | MUERTO | debug: check::tests::tainted_left_does_not_suppress_direct_error_on_right |
| M00022 | src/check/types.rs:9122 | int_inc | 54c9dfed | a45cfbb3 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia; solo cambia el camino `stage == 2 && result.is_err()` de un brazo con guarda (error duro del bloque, p. ej. AnalysisLimit) |
| M00201 | src/check/types.rs:10669 | drop_assignment | 54c9dfed | a7f74c6e | MUERTO | debug: check::tests::adr_gap_array_element_mismatch_uses_element_primary_and_whole_annotation_context |
| M00107 | src/check/types.rs:9796 | and | 54c9dfed | 7e76e10c | MUERTO | debug: check::tests::divergent_left_keeps_independent_numeric_operand_sites |
| M00273 | src/check/types.rs:11048 | false | 54c9dfed | c18b7bc0 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00170 | src/check/types.rs:10326 | false | 54c9dfed | 95d1f8a8 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00015 | src/check/types.rs:9104 | negate_if | 54c9dfed | cfa14969 | MUERTO | debug: ir::lower::lowering_stack_regression::families_lower_with_204_kib |
| M00275 | src/check/types.rs:11056 | drop_assignment | 54c9dfed | e264586a | INCONCLUSO | NO TERMINA: dos ejecuciones en debug (la segunda sin otras cargas del propio carril), 1.500 s cada una (timeout); la suite correcta termina; detectado por no terminacion, sin test que lo nombre |
| M00268 | src/check/types.rs:11020 | drop_assignment | 54c9dfed | e031d0cf | NO COMPILA |  |
| M00134 | src/check/types.rs:9970 | and | 54c9dfed | 12190502 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores (debug, 3 perfiles, check) + 1.285 del corpus (debug)); no se afirma equivalencia |
| M00189 | src/check/types.rs:10500 | drop_return | 54c9dfed | 1188d181 | NO COMPILA |  |
| M00094 | src/check/types.rs:9682 | swap_args | 54c9dfed | 3a5f05f2 | NO COMPILA |  |
| M00317 | src/check/types.rs:11919 | drop_return | 54c9dfed | 80188f4d | MUERTO | debug: nexo::suite$f0005_b1b::f5_declared_constants_remain_pending_at_every_use_site |
| M00290 | src/check/types.rs:11167 | start_end | 54c9dfed | 9a31ec86 | MUERTO | debug: check::tests::adr_gap_primitive_domain_primary_is_first_offending_argument |
| M00316 | src/check/types.rs:11918 | negate_if | 54c9dfed | 54190e26 | MUERTO | debug: ir::lower::tests::f5_neg_reaches_lowering_boundary |
| M00369 | src/check/types.rs:12333 | drop_return | 54c9dfed | 92698f4e | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00353 | src/check/types.rs:12177 | and | 54c9dfed | b98d44fd | NO COMPILA |  |
| M00244 | src/check/types.rs:10864 | drop_assignment | 54c9dfed | 3da9b300 | MUERTO | debug: check::tests::annotation_parameter_and_assignment_contexts_report_concrete_types |
| M00262 | src/check/types.rs:10981 | continue | 54c9dfed | 08a2d0fa | NO COMPILA |  |
| M00377 | src/check/types.rs:12407 | drop_state | 54c9dfed | d029db3b | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00228 | src/check/types.rs:10813 | swap_args | 54c9dfed | 0223c851 | NO COMPILA |  |
| M00381 | src/check/types.rs:12435 | drop_record | 54c9dfed | 32c52610 | MUERTO | debug: nexo::suite$i07c_mutation_regressions::invalid_nested_selector_preserves_independent_len_domain |
| M00222 | src/check/types.rs:10786 | drop_assignment | 54c9dfed | c193e187 | MUERTO | debug: check::tests::conditions_require_bool_and_assignment_keeps_written_type_origin |
| M00366 | src/check/types.rs:12303 | swap_args | 54c9dfed | edbdbefb | NO COMPILA |  |
| M00007 | src/check/types.rs:9015 | false | 54c9dfed | fae9883b | MUERTO | debug: nexo::suite$i07c_mutation_regressions::alias_of_intrinsically_fixed_binding_keeps_no_source_origin |
| M00115 | src/check/types.rs:9851 | ne | 54c9dfed | f6bd0efe | MUERTO | debug: nexo::suite$i07c_causal_obligations::independent_operands_keep_their_causal_sites |
| M00386 | src/check/types.rs:12475 | negate_if | 54c9dfed | 07cb5411 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00161 | src/check/types.rs:10262 | drop_return | 54c9dfed | 11f7d3ea | MUERTO | debug: check::types::quota_tests::boundary_closure_call_restores_parent_fact |
| M00367 | src/check/types.rs:12310 | negate_if | 54c9dfed | ec9a58b7 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00054 | src/check/types.rs:9479 | int_dec | 54c9dfed | 9df7605a | INCONCLUSO |  |
| M00139 | src/check/types.rs:10044 | swap_args | 54c9dfed | aea5d88e | NO COMPILA |  |
| M00042 | src/check/types.rs:9429 | drop_assignment | 54c9dfed | d6a48f35 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00387 | src/check/types.rs:12475 | eq | 54c9dfed | 9de83b79 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00028 | src/check/types.rs:9161 | int_inc | 54c9dfed | 9a94e6a6 | MUERTO | debug: check::types::j16_probe_types::parent |
| M00027 | src/check/types.rs:9157 | int_dec | 54c9dfed | 88f63f9b | INCONCLUSO |  |
| M00195 | src/check/types.rs:10615 | swap_args | 54c9dfed | 9b3b5c48 | NO COMPILA |  |
| M00277 | src/check/types.rs:11063 | drop_assignment | 54c9dfed | af44b36a | NO COMPILA |  |
| M00230 | src/check/types.rs:10817 | drop_state | 54c9dfed | dde4bdc4 | MUERTO | debug: frontend::tests::lexer_and_parser_scale_linearly_across_surface_families |
| M00325 | src/check/types.rs:11970 | drop_return | 54c9dfed | c79f005d | MUERTO | debug: ir::i07c_tests::assert_eq_logical_pair_charge_is_identical_for_live_and_deferred_values |
| M00076 | src/check/types.rs:9547 | int_inc | 54c9dfed | 2665e5f3 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00049 | src/check/types.rs:9461 | true | 54c9dfed | dc07f507 | MUERTO | debug: check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit |
| M00304 | src/check/types.rs:11330 | false | 54c9dfed | b9878f37 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00269 | src/check/types.rs:11029 | drop_assignment | 54c9dfed | 5ab30058 | MUERTO | debug: check::tests::direct_checker_checks_child_of_unavailable_try_operator |
| M00364 | src/check/types.rs:12289 | drop_assignment | 54c9dfed | 5644d094 | MUERTO | debug: nexo::suite$i07c_deferred_mutation_cases::m00453_omits_derived_union |
| M00253 | src/check/types.rs:10904 | drop_assignment | 54c9dfed | - | INCONCLUSO |  |
| M00370 | src/check/types.rs:12340 | negate_if | 54c9dfed | e817811e | MUERTO | debug: check::tests::conditional_selector_constraint_blames_the_later_conflicting_value |
| M00331 | src/check/types.rs:11978 | start_end | 54c9dfed | 9420049c | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00376 | src/check/types.rs:12396 | false | 54c9dfed | 6895ae90 | MUERTO | debug: ir::i07c_tests::divergent_initializer_and_array_operand_follow_their_distinct_constraints |
| M00160 | src/check/types.rs:10237 | int_inc | 54c9dfed | 27076b78 | MUERTO | debug: check::types::quota_tests::boundary_closure_call_restores_parent_fact |
| M00063 | src/check/types.rs:9505 | decrement | 54c9dfed | f23383b5 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00031 | src/check/types.rs:9223 | swap_args | 54c9dfed | 95a5b980 | NO COMPILA |  |
| M00297 | src/check/types.rs:11228 | false | 54c9dfed | 3c87d576 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00013 | src/check/types.rs:9086 | drop_return | 54c9dfed | 9900af97 | MUERTO | debug: nexo::suite$i07c_pattern_deferred::open_pattern_reports_family_at_pattern_and_coverage_at_match |
| M00303 | src/check/types.rs:11268 | negate_guard | 54c9dfed | 3c2ca6d0 | MUERTO | debug: check::tests::adr_gap_non_name_call_target_uses_whole_target_span |
| M00209 | src/check/types.rs:10736 | is_none | 54c9dfed | d5bfe4ba | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00274 | src/check/types.rs:11051 | drop_assignment | 54c9dfed | f113ee4b | MUERTO | debug: check::tests::mode_uses_associated_parameter_not_first_parameter |
| M00324 | src/check/types.rs:11964 | drop_assignment | 54c9dfed | 15217feb | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00194 | src/check/types.rs:10610 | swap_args | 54c9dfed | 9f1008d0 | NO COMPILA |  |
| M00334 | src/check/types.rs:12004 | end_start | 54c9dfed | feb6093c | MUERTO | debug: nexo::suite$j11_generator::generator_question_mark_and_shadowed_iterator_limit_forms |
| M00059 | src/check/types.rs:9493 | drop_assignment | 54c9dfed | 09f6eeb3 | MUERTO | debug: check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit |
| M00075 | src/check/types.rs:9547 | int_dec | 54c9dfed | 753bb29c | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00337 | src/check/types.rs:12056 | sub | 54c9dfed | bd4f53c7 | MUERTO | debug: check::types::quota_tests::non_result_try_is_rejected_at_the_expression_boundary |
| M00165 | src/check/types.rs:10310 | eq | 54c9dfed | 4120c86b | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00365 | src/check/types.rs:12290 | swap_args | 54c9dfed | ba876bd3 | NO COMPILA |  |
| M00159 | src/check/types.rs:10236 | int_inc | 54c9dfed | ec0c770c | MUERTO | debug: check::types::quota_tests::boundary_closure_call_restores_parent_fact |
| M00211 | src/check/types.rs:10746 | is_some | 54c9dfed | e9d4a9c3 | MUERTO | debug: check::tests::written_parameter_result_and_assignment_type_spans_exclude_trivia |
| M00053 | src/check/types.rs:9479 | drop_state | 54c9dfed | 86df538b | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00041 | src/check/types.rs:9428 | drop_assignment | 54c9dfed | 7d9ab781 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00065 | src/check/types.rs:9524 | negate_if | 54c9dfed | de8f74bd | MUERTO | debug: nexo::suite$f0005_b1b::pending_f5_destinations_and_const_have_exact_public_diagnostics |
| M00343 | src/check/types.rs:12112 | drop_state | 54c9dfed | 5b8f6803 | MUERTO | debug: nexo::suite$f0007::try_constrains_result_error_and_never_does_not_constrain_payload |
| M00080 | src/check/types.rs:9556 | decrement | 54c9dfed | 32c015bb | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00190 | src/check/types.rs:10516 | drop_state | 54c9dfed | 6cba9e1f | MUERTO | debug: check::tests::a_returning_if_branch_does_not_contribute_a_value_type |
| M00276 | src/check/types.rs:11057 | continue | 54c9dfed | ab914987 | NO COMPILA |  |
| M00097 | src/check/types.rs:9715 | swap_args | 54c9dfed | cba90850 | NO COMPILA |  |
| M00384 | src/check/types.rs:12465 | swap_args | 54c9dfed | 0e64fccc | NO COMPILA |  |
| M00069 | src/check/types.rs:9528 | int_inc | 54c9dfed | 3a6d9788 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00096 | src/check/types.rs:9713 | and | 54c9dfed | 33d1ca2b | MUERTO | debug: nexo::suite$i07c_causal_obligations::reduced_independent_sites_remain_causal |
| M00157 | src/check/types.rs:10228 | true | 54c9dfed | 2023374d | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00021 | src/check/types.rs:9109 | eq | 54c9dfed | 30f8c2b5 | MUERTO | debug: nexo::suite$j05_match_guards::guard_error_preserves_derived_coverage_and_none_inference |
| M00109 | src/check/types.rs:9814 | and | 54c9dfed | f058bb4f | MUERTO | debug: ir::i07c_tests::bad_binary_left_still_checks_independent_right_branch |
| M00266 | src/check/types.rs:11009 | break | 54c9dfed | e05426ec | INCONCLUSO |  |
| M00110 | src/check/types.rs:9818 | false | 54c9dfed | 6f529ed0 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00003 | src/check/types.rs:8976 | and | 54c9dfed | 89ec7a55 | MUERTO | debug: ir::lower::tests::f5_source_compound_index_evaluates_selector_before_rhs_once |
| M00234 | src/check/types.rs:10832 | drop_assignment | 54c9dfed | 2f2b2c20 | MUERTO | debug: check::types::j16_probe_types::parent |
| M00128 | src/check/types.rs:9922 | swap_args | 54c9dfed | c2e0906d | NO COMPILA |  |
| M00174 | src/check/types.rs:10363 | eq | 54c9dfed | 26148726 | MUERTO | debug: check::tests::adr_gap_result_constructor_payload_label_is_v |
| M00193 | src/check/types.rs:10574 | drop_return | 54c9dfed | d45e983b | NO COMPILA |  |
| M00249 | src/check/types.rs:10895 | drop_assignment | 54c9dfed | - | INCONCLUSO |  |
| M00090 | src/check/types.rs:9650 | is_some | 54c9dfed | 8276c16f | MUERTO | debug: check::tests::array_conflict_types_keep_the_outer_constructor_in_checker_diagnostics |
| M00329 | src/check/types.rs:11978 | start_end | 54c9dfed | e7459869 | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00177 | src/check/types.rs:10417 | drop_assignment | 54c9dfed | 8cf4e6bf | MUERTO | debug: check::tests::adr_gap_primitive_domain_primary_is_first_offending_argument |
| M00400 | src/check/types.rs:12581 | is_none | 54c9dfed | 8d2e3bdf | MUERTO | debug: ir::i07c_tests::adr_gap_nested_empty_arrays_resolve_from_outer_branch_in_both_orders |
| M00171 | src/check/types.rs:10327 | true | 54c9dfed | c00b2f7a | SUPERVIVIENTE | SIN PROGRAMA tras la caza (860 dirigidas + 600 combinatorias + 192 de profundidad + 210 de selectores + 1.285 del corpus, binarios debug, 3 perfiles, check); deuda 2026-11-24, no se afirma equivalencia |
| M00120 | src/check/types.rs:9863 | and | 54c9dfed | dcc0c397 | MUERTO | debug: check::tests::all_preparsed_names_are_resolved_by_lexical_position |
| M00285 | src/check/types.rs:11096 | true | 54c9dfed | b3fe4ef7 | MUERTO | debug: check::tests::annotation_parameter_and_assignment_contexts_report_concrete_types |
| M00154 | src/check/types.rs:10194 | true | 54c9dfed | 4d8fa299 | MUERTO | debug: check::tests::argument_labels_select_types_without_changing_written_diagnostic_order |

