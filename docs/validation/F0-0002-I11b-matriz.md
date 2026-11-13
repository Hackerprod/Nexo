# F0-0002 I11b — matriz de validación

Fecha: 2026-11-12. Clasificación y alcance conforme a F0-0002 §§17–18. Los comandos de formato, Clippy y suites completas se reconstruyen en el clon limpio; la matriz registra también las verificaciones dirigidas ya ejecutadas.

| Requisito | Clase | Evidencia y referencia al ADR | Veredicto |
|---|---|---|---|
| V01 — alcance | Núcleo | `tests/corpus.rs::corpus_has_only_official_source_extension`; inventario de fuentes y búsquedas de sintaxis histórica en `src/`, `tests/` y `corpus/`; pruebas CLI de `corpus/pass` y `corpus/fail` en los dos perfiles. Solo queda activa la sintaxis B de D15. D15 §§1, 3.6, 6.1 y 7.1; F0-0002 §18 V01. | PASA |
| V02 — frontend | Núcleo | Suite completa debug/release; `token_sequence_mutations_never_panic_and_accepted_asts_reparse` en Release con semilla 2026111201 y 20.000 casos; negativos del corpus conservan token y span. D15 §§2–5; F0-0002 §§3, 13.14 y 18 V02. | PASA |
| V03 — llamadas | Núcleo | Suite completa debug/release y `tests/cli.rs`; `parenthesized_indexed_places_run_with_expected_output` verifica seis lugares indexados parentizados con `check`/`run`, incluidas llamadas `edit`. F0-D15 §3.1; F0-0002 §§4.2–4.5, 14.1 y 18 V03. | PASA |
| V04 — propiedad | Núcleo | Suite completa debug/release; `tests/corpus.rs` y regresiones I07a/I07b/I07c; RD normativo 152/152 por perfil, incluidos diagnósticos causales. F0-0002 §§4.1.1–4.1.2, 6, 9.2, 14.2, 14.10 y 18 V04. | PASA |
| V05 — préstamos | Núcleo | Suite completa debug/release; pruebas I06 de solapamiento, représtamo, permisos y no escape; flujos de recursos del corpus. F0-0002 §§5.1–5.4, 14.2 y 18 V05. | PASA |
| V06 — `Result` y `?` | Núcleo | `tests/cli.rs::assert_eq_result_constructor_uses_first_operand_context` y `tests/i08.rs::{assert_eq_first_result_type_context_resolves_ok_component,assert_eq_intrinsic_result_context_resolves_ok_component}` verifican que el primer argumento de `AssertEq` determina `Ok`; `deferred_result_alias_conflict_pins_current_t01_diagnostic` conserva A/T01. Siguen fallando contextos posteriores por anotación, argumento y retorno, documentados abajo; V06 no pasa. F0-0002 §§4.1.1, 4.6, 7.1–7.2, 13.17 y 18 V06. | FALLA |
| V07 — IR | Núcleo | Suite completa debug/release; `parenthesized_indexed_places_run_with_expected_output` verifica seis formas de lugar con paréntesis en `check`/`run`; `parenthesized_places.rs` fija el diagnóstico de tipo y el permiso inmutable. F0-D15 §3.1; F0-0002 §§9.1–9.4, 15.4 y 18 V07. | PASA |
| V08 — efectos | Núcleo | Suite completa debug/release; campaña I09 ignorada en Release; resúmenes transitivos y recomprobación después del lowering. F0-0002 §§8.1–8.8, 14.4 y 18 V08. | PASA |
| V09 — cuotas, trabajo y bytes | Andamiaje | Pruebas de cuotas debug/release, perfiles reducidos, limpieza tras agotamiento y ticks normativos 2/6/1; campaña I10b ignorada en Release. Once ensayos de escala que originalmente rebasaban `ir_items=262144` o `ownership_flow_steps=1000000` se ajustaron a fuentes de 32/128 KiB (16/64 KiB para escalares) y conservan la razón 4×; el perfil normal no se amplió. Ocho observaciones siguen fuera de umbrales locales no publicados por el ADR: siete mediciones de escala y una razón temporal; constan en `Pending.md` y no producen una salida semántica incorrecta. F0-0002 §§11.1–11.8, 13.13 y 18 V09. | PASA |
| V10 — diagnósticos | Núcleo | `tests/corpus.rs::negative_corpus_exact_codes` comprueba código/subcausa y span; la prueba de paridad CLI exige que check/run coincidan en diagnóstico y que debug/release den bytes idénticos; RD 152/152 por perfil. F0-0002 §§13.6–13.9, 13.13, 14 y 18 V10. | PASA |
| V11 — flujos | Andamiaje | Suite debug/release y campañas I11a ignoradas de runtime/colas en Release; corpus CLI cruzado entre perfiles. Se comprueban EX01, JSON, rutas y fallos variables. F0-0002 §§12.1–12.4, 17 fila I11 y 18 V11. | PASA |
| V12 — independencia | Andamiaje | `scripts/i07c_rd_conformidad.py --verify` contra ambos binarios: 152/152 pares check/run, cero diferencias de instantánea; campañas existentes de parser/JSON/argv y diferencial mock completadas. La matriz separa las regresiones internas Rust de IR de los arneses Python; no se generaron poblaciones de mutantes ni se lanzaron campañas nuevas de mutación de IR o de Python. F0-0002 §§15.1–15.4, 18 V12. | PASA |
| V13 — conservación | Andamiaje | `docs/baseline/` y `Claude Agent.cmd` intactos; las fuentes del corpus tienen extensión `.n`; `26_ex01_herramienta.n`, `27_ex02_http_json.n` y `28_ex03_simulacion.n` conservan grafías de sintaxis A y no ejecutan, tal como registra `corpus/objetivo/INDEX.md`. `Pending.md` conserva visibles las observaciones abiertas. SHA-256 tras normalizar LF, comparado con blob Git: `Nexo_CAMBIOS_rev0.2.md` `118c00274155f0a5a8534b6230ce2e8f079bd9829598aa28336cfa1ff3d38a8c`, `Nexo_ROADMAP_rev0.2.md` `9a7ed7336a920f82bfcbd92030761320e4e7b334991a95acdc1ee5252144d62d`. F0-0002 §§1, 13.4, 17–18. | PASA |

## Diferenciales de corpus y flujos

Se añadió una sola prueba a `tests/corpus.rs`: `corpus_cli_check_run_diagnostics_match_across_profiles`. Para los 5 programas de `corpus/pass` y los 10 de `corpus/fail`, ejecuta `check` y `run` en el binario del perfil actual y en el otro perfil. Debug↔Release y check↔run conservan códigos de salida; en negativos el texto JSON completo coincide, incluidos código, subcausa, spans primarios y campos. La prueba pasó en ambos sentidos: Release contra Debug y Debug contra Release.

Fuzz Release: parser 20.000 casos (semilla 2026111201), JSON del entorno mock 20.000 (2026111202), argv 20.000 (2026111203) y diferencial mock 100.000 (2026111204). Resultado observado: cero pánicos, `internal_error`, `InvalidIr` o cuelgues. La matriz CLI totalizó 21.904 invocaciones y coincidió con el oráculo D5/D15/A2.

## Mutaciones internas de IR y arneses Python

- Las regresiones internas de IR son tests Rust (`src/ir/i0x_*`, `src/ir/i07c_*` y `tests/i07c_mutation_regressions.rs`). Las suites normales debug/release ejecutan sus regresiones; I11b no genera poblaciones internas de mutantes.
- Los programas de `scripts/*.py` son herramientas independientes. `scripts/i07c_rd_conformidad.py` se usó como diferencial normativo RD en ambos perfiles; no se ejecutó ninguna campaña nueva de mutación Python ni `scripts/i11a_mutaciones.py`.
- La campaña existente `mock_oracle_exhaustive_document_mutation_campaign` es una prueba Rust del JSON mock, distinta de las dos clases anteriores; pasó junto con los demás tests ignorados de integración.

## Observaciones de escala no bloqueantes

Los tests ignorados se inventariaron por nombre y motivo más abajo. Las observaciones reproducibles que fallan solo razones locales de trabajo, tiempo o asignaciones, sin cifra correspondiente en el ADR ni salida incorrecta de CLI, constan como filas NB-I11b en `Pending.md`. B1 quedó corregido y tiene regresiones. B2 conserva tres repros posteriores de anotación, argumento y retorno que producen `NX-E0113` sobre `Ok`.

## Inventario de pruebas `#[ignore]`

| Test ignorado | Motivo documentado |
|---|---|
| `src/check/i07c_quota_campaign.rs::reduced_quota_property` | reduced quota property campaign; run explicitly with I07C_QUOTA_SEEDS |
| `src/check/types.rs::generated_multifunction_quota_has_one_denied_charge_and_exact_prefix` | three-function reduced type-solver quota campaign |
| `src/frontend/tests.rs::omitted_else_parenthesis_comment_gaps_stabilize` | else parentheses and comment gap campaign |
| `src/frontend/tests.rs::consecutive_comment_gaps_preserve_canonical_properties` | consecutive comments across omitted-token gaps |
| `src/frontend/tests.rs::omitted_token_generated_programs` | generated token-origin campaign |
| `src/frontend/tests.rs::accepted_commented_sources_keep_run_result_after_format` | large accepted commented execution equivalence campaign |
| `src/frontend/tests.rs::generated_assignment_places_and_comments_preserve_canonical_tree` | large assignment and comment matrix |
| `src/frontend/tests.rs::generated_comment_positions_preserve_ast_order_and_idempotence` | large directed comment campaign |
| `src/frontend/tests.rs::every_concrete_gap_preserves_comments_and_tree` | systematic F0 comment gap campaign |
| `src/frontend/tests.rs::generated_valid_programs_round_trip` | campaña extensa de ida y vuelta; ejecutar en batería medida |
| `src/frontend/tests.rs::mutated_bytes_and_tokens_never_panic_or_emit_invalid_diagnostics` | campaña extensa de bytes y tokens; ejecutar en batería medida |
| `src/frontend/tests.rs::token_sequence_mutations_never_panic_and_accepted_asts_reparse` | campaña extensa de secuencias de tokens; ejecutar en batería medida |
| `src/frontend/tests.rs::printer_and_formatter_scale_in_emitted_and_allocated_bytes` | escala extensa de impresor y formateador; ejecutar en batería medida |
| `src/ir/effects.rs::witness_search_release_time_ratio_is_bounded` | release timing campaign; record locally when running scale verification |
| `src/ir/effects.rs::witness_release_timing_matches_validator_scale_shapes` | release timing repro for validator B1 |
| `src/ir/i05_tests.rs::generated_calls_resources_and_early_returns_match_independent_reference` | campaña extensa de llamadas y recursos; ejecutar en batería medida |
| `src/ir/i05_tests.rs::i05_call_and_resource_source_families_scale_linearly` | explicit 256 KiB to 1 MiB measurement |
| `src/ir/i05_tests.rs::i05_large_signature_and_many_functions_scale_linearly` | explicit 256 KiB to 1 MiB lowering measurement |
| `src/ir/i05_tests.rs::i05_unreachable_and_dense_declarations_scale_across_twelve_families` | normal-profile dead-code and dense-declaration scale measurement |
| `src/ir/i06_tests.rs::generated_inout_and_copy_programs_match_independent_integer_model` | campaña extensa de edit y copia; ejecutar en batería medida |
| `src/ir/i06_tests.rs::semantic_loan_fuzz_never_constructs_invalid_ir` | campaña extensa de préstamos semánticos; ejecutar en batería medida |
| `src/ir/i06_tests.rs::loan_source_fuzz_validates_diagnostics_and_never_panics` | campaña extensa de fuentes con préstamos; ejecutar en batería medida |
| `src/ir/i06_tests.rs::dense_loan_and_call_sources_scale_across_twelve_families` | normal-profile 32 KiB to 128 KiB compilation measurement |
| `src/ir/i06_tests.rs::sequential_operands_never_escape_as_invalid_ir` | structured sequential-operand ownership campaign |
| `src/ir/i07a_tests.rs::loop_backedges_scale_in_steps_allocation_and_time` | normal-profile 32 KiB to 128 KiB verifier loop scale measurement |
| `src/ir/i07a_tests.rs::directed_divergence_positions_match_reference_acceptance_and_diagnostics` | explicit 200000-program divergence campaign with template oracle |
| `src/ir/i07a_tests.rs::generated_cfg_programs_match_independent_integer_model` | campaña extensa de CFG; ejecutar en batería medida |
| `src/ir/i07a_tests.rs::semantic_cfg_sources_never_create_invalid_ir` | campaña extensa de fuentes CFG; ejecutar en batería medida |
| `src/ir/i07a_tests.rs::cfg_dense_sources_have_bounded_deterministic_work` | normal-profile 32 KiB to 128 KiB CFG work measurement |
| `src/ir/i07a_tests.rs::cfg_later_operands_scale_linearly_in_work_and_allocation` | normal-profile 32 KiB to 128 KiB later-operand CFG scale measurement |
| `src/ir/i07a_tests.rs::injected_divergence_matches_live_twin_diagnostics` | explicit 300000-program differential divergence campaign |
| `src/ir/i07a_tests.rs::divergent_and_live_sources_scale_in_work_bytes_and_allocations` | normal-profile 32 KiB to 128 KiB divergent and live scale measurement |
| `src/ir/i07b_tests.rs::ownership_quota_large_source_keeps_converged_loop_diagnostic` | explicit declarations-first 820 KiB diagnostic-pass quota regression |
| `src/ir/i07b_tests.rs::shared_node_nested_condition_matches_dense_reference` | explicit 100000-program shared-node nested-condition sparse/dense comparison |
| `src/ir/i07b_tests.rs::consumed_prefix_loop_suffix_scales_across_source_budget` | normal-profile consumed-prefix loop-suffix scale measurement |
| `src/ir/i07b_tests.rs::sparse_ownership_scales_across_fourteen_adversarial_families` | normal-profile adversarial sparse-state scale measurement |
| `src/ir/i07b_tests.rs::extensive_nested_loop_ownership_matches_source_ast_model` | explicit 200000-program source-AST ownership campaign |
| `src/ir/i07b_tests.rs::generated_sparse_ownership_matches_dense_reference_exactly` | explicit 100000-program sparse versus dense ownership comparison |
| `src/ir/i07b_tests.rs::loop_fixpoint_scales_by_steps_and_allocation_traffic` | normal-profile 32-KiB to 128-KiB CFG scale measurement |
| `src/ir/i07c_tests.rs::generated_expected_context_matches_recursive_leaf_model` | 100000 unique expected-type programs checked against an independent leaf model |
| `src/ir/i07c_tests.rs::generated_type_permutations_preserve_acceptance` | 300000 generated sources with branch, equality, array, declaration and alias permutations |
| `src/ir/i07c_tests.rs::statement_permutations_match_independent_constraint_oracle` | 150000 constraint programs in four statement orders |
| `src/ir/i07c_tests.rs::extensive_monomorphic_constraints_match_independent_graph_oracle` | exact 200000-pair constraint graph campaign; run explicitly |
| `src/ir/i07c_tests.rs::deferred_unification_scale_is_linear_in_steps_and_allocations` | exact 15-family 256 KiB to 1 MiB deterministic scale campaign |
| `src/ir/i07c_tests.rs::nested_assert_eq_unification_scale_tracks_depth_and_source_bytes` | nested AssertEq depth and 256 KiB to 1 MiB deterministic scale campaign |
| `src/ir/i07c_tests.rs::extensive_structured_deferred_check_and_run_match_live_twins` | 750000-source structured check/run live/dead/array campaign |
| `src/ir/i10a_tests.rs::verifier_effect_release_time_campaign_records_branch_ratios` | release timing campaign; t(24000)/t(3000) takes over four seconds locally |
| `src/ir/tests.rs::access_prefix_contexts_never_escape_as_invalid_ir` | directed prefix and expression-context campaign |
| `src/ir/tests.rs::unicode_sources_never_reach_internal_failure` | Unicode source and lowering campaign |
| `src/ir/tests.rs::function_and_other_nonlocal_assignment_fuzz_has_no_internal_error` | directed rejected-place campaign |
| `src/ir/tests.rs::indexed_place_effects_never_escape_as_invalid_ir` | structured indexed-place ownership campaign |
| `src/ir/tests.rs::array_ir_lowering_scales_across_ten_source_families` | explicit array lowering scale measurement |
| `src/ir/tests.rs::scalar_ir_lowering_scales_across_fifteen_source_families` | normal-profile 16 KiB to 64 KiB scalar lowering scale measurement |
| `src/ir/work_oracle_tests.rs::generated_sources_match_owned_tree_cost_oracle` | campaign of independently generated source trees |
| `tests/i07c_deferred_metamorphic.rs::poisoned_assignment_and_independent_let_have_identical_rhs_causes` | 50,000-program metamorphic campaign; run explicitly in debug and release |
| `tests/i07c_deferred_metamorphic.rs::later_array_assignment_and_concrete_initializer_have_identical_causes` | campaña de gemelos de 300.000 fuentes; ejecutar por perfil |
| `tests/i07c_mutant_projection_scale.rs::resolved_projection_does_not_repartition_unrelated_pending_constraints` | escala de asignaciones tras resolver proyección y sin proyecciones |
| `tests/i08_campaign.rs::generated_result_programs_preserve_check_run_and_cleanup` | campaña tipada I08; ejecutar explícitamente en debug y release |
| `tests/i08_robustness.rs::mutated_result_sources_do_not_panic_or_reach_invalid_ir` | sonda determinista de fuentes Result mutadas |
| `tests/i09_campaign.rs::typed_effect_programs_keep_contracts_and_transitive_summaries` | typed I09 campaign; run explicitly in debug and release |
| `tests/i10b_campaign.rs::generated_work_counters_match_the_independent_loop_ledger` | I10b deterministic runtime work campaign; run explicitly in debug and release |
| `tests/i10b_campaign.rs::runtime_work_time_ratio_uses_a_calibrated_threshold` | release timing campaign; calibrates runtime work ratio against a linear control |
| `tests/i11a.rs::i11a_seeded_runtime_campaign` | deterministic I11a campaign; execute with scripts/i11a_campanas.py |
| `tests/i11a.rs::i11a_seeded_mixed_queue_campaign` | deterministic I11a mixed queue campaign; execute with scripts/i11a_campanas.py |
| `tests/i11a_val58.rs::normal_ir_item_limit_is_profiled_and_reports_the_declared_budget` | normal-profile ir_items source witness; execute through scripts/i11a_campanas.py |
| `tests/mock_oracle.rs::mock_oracle_exhaustive_document_mutation_campaign` | exhaustive byte and structural-sequence mutations over valid mock documents |
| `tests/mock_oracle.rs::mock_oracle_work_cells_priority_probe` | probe no normativo: A2 no ordena el agotamiento simultáneo de trabajo y celdas |
| `tests/mock_oracle.rs::mock_oracle_differential_campaign` | seeded independent I11a parser/runtime differential campaign |

## Cierre integrado y reparaciones

`sh scripts/validate.sh` pasó en un clon limpio con `target` vacío: código 0; `cargo fmt --all -- --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, suite debug (1543 aprobadas, 0 fallidas, 67 ignoradas) y release (1545, 0, 68). La única ejecución limpia de Clippy encontró `needless_range_loop` en el test de paridad; se corrigió iterando por `zip`, y la validación final quedó verde. La regresión es `corpus_cli_check_run_diagnostics_match_across_profiles`.

Por separado, `scripts/i07c_rd_conformidad.py --verify` ejecutó RD (152/152 por perfil). En Release se ejecutaron las 68 pruebas ignoradas por familia. Las 14 de integración pasaron. De las 54 de biblioteca, tras ajustar los testigos de escala que no cabían en el perfil normal y repetir las familias afectadas, siete pruebas siguen fallando solo umbrales locales de bytes/trabajo/pasos y una razón temporal I10a; se registran en Pending como NB-I11b-01..08.

En el mismo clon limpio, la regresión de paridad pasó dos veces: debug contra release y release contra debug. `scripts/i07c_rd_conformidad.py --verify` dio 152/152 check/run y cero diferencias de snapshot en ambos binarios. El objetivo se regeneró: 10/29 programas ejecutan.

El regenerador `scripts/objetivo.py --actualizar` reemplaza ahora solo la tabla delimitada por sus marcadores y preserva el apartado existente de traducciones pendientes. Se comprobó al restaurar el índice base y regenerar 10/29 sin perder dicho apartado.

## Defectos de núcleo y alcance pendiente

- B1 (V03/V07) queda cubierto por `tests/cli.rs::parenthesized_indexed_places_run_with_expected_output` y `tests/parenthesized_places.rs::{parenthesized_indexed_place_type_error_keeps_exact_diagnostic,parenthesized_immutable_place_keeps_exact_diagnostic}`. Las seis variantes válidas se aceptan con `check` y `run`; la primera imprime `[2]` y termina con `nexo-result:()`. Los dos negativos conservan código, subcausa y span en ambos comandos.
- B2 (V06) se corrigió para el contexto inmediato que `AssertEq` deriva de su primer operando: `tests/i08.rs::{assert_eq_first_result_type_context_resolves_ok_component,assert_eq_intrinsic_result_context_resolves_ok_component}`. Siguen reproduciéndose `let r=Ok(1); let s:Result<I64,Bool>=r;`, el paso de `r` a un parámetro `view Result<I64,Bool>` y el retorno de `x=Ok(1)` como `Result<I64,I64>`; cada caso da `NX-E0113` en el constructor con `unresolved_components=["$.err"]`. La unificación diferida actual no transfiere el tipo estructurado parcial de un alias `Result` a una restricción posterior; resolverlo requiere decidir si se amplía la política monótona de fijación por orden de fuente. Se documenta como decisión de diseño pendiente, no como conformidad. A/T01 sigue fijado por `tests/i08.rs::deferred_result_alias_conflict_pins_current_t01_diagnostic`.

## Observaciones posteriores a la matriz

`Pending.md` registra además NB-I11b-09 a NB-I11b-11: el cuelgue de la CLI con el nombre reservado de dispositivo de consola en Windows, una intermitencia no reproducible de `nexo check` sobre ficheros recién escritos y el defecto de núcleo heredado de la inferencia diferida de constructores `Result` (V06), que queda fuera del alcance de este corte.
