# NX-build — mediciones de puertas y selección

Fecha: 2026-11-19. Base de código `ae9130f`; la copia medida usa `457ade7`, cuyo único diff frente a `ae9130f` es este plan documental. Rust 1.99.1 MSVC, cargo-nextest 0.9.152, Python 3.13.1, `CARGO_BUILD_JOBS=6`, `CARGO_TARGET_DIR` propio en `C:/nexo-work/tmp/`, `--locked --offline`. Máquina compartida con una campaña de mutación; todos los relojes son muestras bajo esa carga, no cotas estables. Los 72/73 ignorados no se ejecutan.

## Tope por test (decisión del coordinador posterior al plan r2)

`default`: `slow-timeout = { period = "30m", terminate-after = 1 }` (1.800 s). El máximo de las tres muestras de cualquier test en las tablas es 414,0 s en debug y 269,3 s en release; 1.800 s supera 3 × 414,0 s (1.242 s), con un factor 4,35. `fast`: 8 min (480 s) para acotar el bucle de edición; los tests no excluidos más lentos pueden acercarse bajo carga, por lo que un timeout se investiga y se repite aislado. Ambos perfiles carecen de `retries`: un timeout falla la puerta y nunca cuenta como PASS. Un cuelgue aislado de `nexo.exe check` en una muestra caliente fast motivó esta decisión; el test y el mismo comando aislados terminaron normalmente, así que esa muestra se descarta y se repite.

La sintaxis se verificó contra `cargo-nextest 0.9.152` instalado y su `repo-config-reference.md`. Una crate sintética fuera del repositorio (`nx_timeout_probe`, test que duerme 4 s, copia de configuración con periodo reducido a 1 s y `terminate-after = 1`) produjo `TIMEOUT [1.019s]`, resumen `0 passed, 1 timed out`, exit 100 y ningún proceso de prueba restante. Se borrará esa copia al cerrar el corte.

## Inventario reproducible de la base

| Perfil | Descubiertos | Ignorados | Ejecutables | Binarios |
| --- | ---: | ---: | ---: | ---: |
| debug | 2051 | 72 | 1979 | 71 |
| release | 2054 | 73 | 1981 | 71 |

Release añade dos ejecutables y un ignorado mediante `#[cfg(not(debug_assertions))]`: `semantic_passes_scale_with_source_bytes`, `parser_token_patterns_scale_with_source_length` y `verifier_effect_release_time_campaign_records_branch_ratios` (ignorado). Ninguno se excluye en fast. Comandos: `cargo nextest list --locked --offline --profile default --message-format json` con y sin `--release`.

## Corridas completas de nextest en target caliente, antes de NX-B1

`NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --locked --offline --profile default --message-format libtest-json` (añadiendo `--release` para release); salida terminal de cada test, no avisos de >60 s. Cada corrida terminó con todos los ejecutables `ok`.

| Perfil | Pared r1/r2/r3 (s) | Mediana (s) | CPU aparente r1/r2/r3 (s) |
| --- | --- | ---: | --- |
| debug | 407.5 / 451.6 / 437.3 | 437.3 | 5015.7 / 5179.8 / 5674.5 |
| release | 265.5 / 259.2 / 328.6 | 265.5 | 3834.2 / 3666.2 / 5587.2 |

La CPU aparente es la suma de duraciones individuales; no se divide por la pared de una corrida paralela. Las cifras de 1.958 tests/1.958 s de `98ff91d` son históricas y no se mezclan con este inventario.

## Top 60 por perfil (unión de 71 pares)

Cada fila lleva rango debug/release (`—` si queda fuera del top 60), mediana y las tres muestras en segundos. El cometido se cotejó con la función indicada en `origen`; `pesado` significa que se conserva en la puerta completa y se excluye solo en fast.

| D# | R# | binary_id | test_name | debug med (r1/r2/r3) | release med (r1/r2/r3) | origen/corte | cometido | decisión |
| ---: | ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `nexo` | `ir::i10a_tests::ir_item_limit_is_deferred_behind_ownership_and_effect_errors` | 381.6 (376.4/414.0/381.6) | 215.3 (215.3/215.9/68.1) | `src/ir/i10a_tests.rs:508` (I10A) | ir item limit is deferred behind ownership and effect errors | fast: testigo de cuota o diagnóstico |
| 2 | 43 | `nexo` | `ir::i07a_tests::cfg_local_branch_scale_uses_sparse_temp_counters` | 290.2 (290.2/315.0/282.4) | 11.4 (11.4/13.3/9.4) | `src/ir/i07a_tests.rs:1977` (I07A) | barrido de CFG 64/256 KiB | pesado: barrido de CFG 64/256 KiB; if_values_after_other_operands_remain_available_across_cfg_edges |
| 3 | 4 | `nexo` | `ir::i07b_tests::ir_item_limit_rejects_the_first_binding_over_the_compilation_cap` | 256.5 (256.5/257.5/234.5) | 148.0 (153.1/148.0/42.1) | `src/ir/i07b_tests.rs:958` (I07B) | ir item limit rejects the first binding over the compilation cap | fast: testigo de cuota o diagnóstico |
| 4 | 19 | `nexo` | `ir::i07b_tests::consumed_prefix_then_loop_suffix_converges_before_each_exit` | 224.9 (224.9/231.0/202.6) | 71.1 (71.1/74.4/26.2) | `src/ir/i07b_tests.rs:926` (I07B) | barrido de 700–3000 consumos y bucles | pesado: barrido de 700–3000 consumos y bucles; loop_exit_lists_all_reaching_consumptions_and_backedge |
| 5 | 1 | `nexo` | `ir::tests::generated_scalar_programs_match_independent_decimal_model` | 186.0 (195.1/186.0/175.2) | 215.4 (235.1/215.4/16.9) | `src/ir/tests.rs:3712` (IR general) | 8.000 programas aritméticos | pesado: 8.000 programas aritméticos; f5_operator_tests::integer_results_match_separate_arbitrary_precision_oracle |
| 6 | 2 | `nexo` | `ir::tests::generated_array_copies_and_writes_match_independent_matrix_model` | 185.9 (195.1/185.9/172.9) | 215.4 (234.5/215.4/16.9) | `src/ir/tests.rs:3559` (IR general) | 8.000 matrices de arrays | pesado: 8.000 matrices de arrays; ordinary_array_parameter_returns_deep_copy y selected_array_copy_is_independent_of_mutable_root |
| 7 | 51 | `nexo` | `ir::i07a_tests::later_operand_dominance_verification_scales_by_cfg_work` | 162.5 (165.3/162.5/150.8) | 7.0 (7.0/8.2/3.0) | `src/ir/i07a_tests.rs:1954` (I07A) | barrido de dominancia 64/128 KiB | pesado: barrido de dominancia 64/128 KiB; if_values_after_other_operands_remain_available_across_cfg_edges |
| 8 | 6 | `nexo` | `ir::i07a_tests::directed_branch_error_and_divergence_fuzz_has_no_internal_failure` | 138.8 (138.8/140.2/118.1) | 128.0 (128.4/128.0/9.0) | `src/ir/i07a_tests.rs:1758` (I07A) | fuzz dirigido de ramas | pesado: fuzz dirigido de ramas; directed_divergence_positions_match_reference_acceptance_and_diagnostics |
| 9 | 8 | `nexo` | `ir::work_oracle_tests::owned_source_tree_oracle_covers_catalog_work_rows` | 132.4 (132.4/141.6/108.3) | 122.5 (122.5/164.8/6.6) | `src/ir/work_oracle_tests.rs:253` (IR general) | owned source tree oracle covers catalog work rows | fast: oráculo singular |
| 10 | 5 | `nexo` | `frontend::tests::canonical_printer_preserves_full_b_ast_across_body_and_signature_matrix` | 130.1 (130.1/135.4/108.7) | 132.7 (134.6/132.7/10.9) | `src/frontend/tests.rs:3674` (D15/frontend) | matriz B de impresor | pesado: matriz B de impresor; accepted_b_ast_round_trips_without_source_spans |
| 11 | 9 | `nexo` | `ir::i07b_tests::generated_sparse_ownership_matches_dense_reference_in_regular_tests` | 127.8 (127.8/127.9/112.4) | 121.5 (126.5/121.5/5.8) | `src/ir/i07b_tests.rs:1424` (I07B) | modelo generado disperso/denso | pesado: modelo generado disperso/denso; shared_node_nested_condition_matches_dense_reference |
| 12 | 10 | `nexo` | `ir::i07b_tests::generated_nested_loop_ownership_matches_source_ast_model` | 127.6 (127.6/128.1/112.8) | 121.3 (126.5/121.3/5.8) | `src/ir/i07b_tests.rs:1419` (I07B) | modelo generado de propiedad en bucles | pesado: modelo generado de propiedad en bucles; condition_arm_effects_cover_values_branches_and_use_positions |
| 13 | 27 | `nexo` | `frontend::tests::lexer_and_parser_scale_linearly_across_surface_families` | 123.7 (123.7/124.4/106.0) | 32.0 (32.0/39.1/13.9) | `src/frontend/tests.rs:4099` (D15/frontend) | barrido léxico y parser | pesado: barrido léxico y parser; published_syntax_rows_have_exact_tokens_spans_and_prefixes |
| 14 | 7 | `nexo` | `ir::tests::array_sources_with_mutated_syntax_never_panic_and_validate_rejections` | 117.6 (117.6/123.2/106.8) | 126.1 (126.1/145.2/5.5) | `src/ir/tests.rs:3589` (IR general) | campaña de arrays mutados | pesado: campaña de arrays mutados; heterogeneous_array_rejection_has_homogeneous_positive y malformed_source_campaign_never_panics_and_all_diagnostics_validate |
| 15 | 12 | `nexo` | `check::tests::valid_generated_names_shadowing_and_comparisons_have_no_diagnostics` | 115.6 (115.6/120.8/90.9) | 106.7 (106.7/113.3/11.4) | `src/check/tests.rs:2994` (checker) | fuentes generadas de nombres | pesado: fuentes generadas de nombres; duplicate_binding_and_inner_shadowing_are_lexical y comparisons_use_right_operand_for_mismatch_and_operator_for_domain |
| 16 | 22 | `nexo::mock_oracle` | `mock_oracle_runtime_contract` | 98.5 (98.5/108.8/65.3) | 64.6 (64.6/108.9/12.3) | `tests/mock_oracle.rs:3138` (A2/F0) | mock oracle runtime contract | fast: oráculo singular |
| 17 | 15 | `nexo::j14_s4_depth` | `deeply_nested_owned_resource_reports_one_depth_error` | 88.5 (88.5/103.9/65.9) | 82.8 (86.7/82.8/48.5) | `tests/j14_s4_depth.rs:422` (J14) | deeply nested owned resource reports one depth error | fast: sonda/frontera de pila o profundidad |
| 18 | 16 | `nexo` | `ir::effects::witness_differential_tests::dynamic_witness_matches_reference_with_variable_leaf_chains_and_sccs` | 86.9 (86.9/90.6/60.9) | 77.2 (83.3/77.2/7.5) | `src/ir/effects.rs:4448` (IR general) | cadenas/SCC de testigos | pesado: cadenas/SCC de testigos; dynamic_witness_matches_simple_path_enumerator_on_small_graphs |
| 19 | 17 | `nexo` | `ir::i07a_tests::mutated_cfg_sources_keep_rejections_structured` | 81.7 (81.7/84.1/56.4) | 74.5 (81.7/74.5/8.8) | `src/ir/i07a_tests.rs:1730` (I07A) | campaña de CFG mutado | pesado: campaña de CFG mutado; cfg_verifier_rejects_bad_edges_branch_type_and_missing_join_definition |
| 20 | 20 | `nexo` | `ir::tests::deepest_admitted_scalar_families_compile_on_small_caller_stacks` | 80.9 (80.9/76.5/87.1) | 66.5 (74.7/66.5/4.1) | `src/ir/tests.rs:990` (IR general) | sonda de pila: deepest admitted scalar families compile on small caller stacks | fast: sonda/frontera de pila o profundidad |
| 21 | 18 | `nexo` | `ir::i05_tests::malformed_call_and_resource_sources_never_panic_and_diagnostics_validate` | 78.7 (78.7/83.7/51.0) | 72.7 (79.2/72.7/6.8) | `src/ir/i05_tests.rs:804` (I05) | malformed call and resource sources never panic and diagnostics validate | fast: caso normativo o sin sustituto corto acreditado |
| 22 | — | `nexo` | `ir::i07a_tests::repeated_while_preheaders_compile_at_two_source_sizes` | 74.7 (75.5/74.7/53.2) | 2.5 (2.1/2.5/2.9) | `src/ir/i07a_tests.rs:355` (I07A) | barrido de preheaders | pesado: barrido de preheaders; while_backedge_rechecks_complete_branching_condition |
| 23 | — | `nexo` | `ir::i07b_tests::repeated_loop_analysis_has_linear_step_growth` | 73.8 (77.4/73.8/50.9) | 2.5 (2.2/2.5/5.8) | `src/ir/i07b_tests.rs:908` (I07B) | barrido de pasos de bucle | pesado: barrido de pasos de bucle; while_condition_effects_reach_body_and_false_exit |
| 24 | 34 | `nexo` | `ir::i05_tests::unreachable_lowering_short_scale_regression` | 69.1 (69.1/72.5/48.6) | 15.9 (15.9/19.0/6.8) | `src/ir/i05_tests.rs:1023` (I05) | escala: unreachable lowering short scale regression | fast: caso normativo o sin sustituto corto acreditado |
| 25 | 21 | `nexo::runtime_profiles_and_resource_limits` | `runtime_profile_table_drives_work_frames_cells_and_observation_boundaries` | 63.5 (58.2/63.5/88.9) | 64.9 (64.9/30.7/119.4) | `tests/runtime_profiles_and_resource_limits.rs:750` (A2/F0) | runtime profile table drives work frames cells and observation boundaries | fast: caso normativo o sin sustituto corto acreditado |
| 26 | — | `nexo` | `ir::i07b_tests::consumed_bindings_before_many_small_loops_keep_sparse_state_linear` | 60.5 (60.5/62.1/41.1) | 4.2 (3.6/4.2/5.9) | `src/ir/i07b_tests.rs:1025` (I07B) | barrido de estado disperso | pesado: barrido de estado disperso; loop_exit_lists_all_reaching_consumptions_and_backedge |
| 27 | 23 | `nexo::j09_closure_fuzz` | `nested_capture_permissions_and_union_carriers_cross_product` | 53.4 (53.4/51.5/55.7) | 51.7 (55.5/51.7/2.8) | `tests/j09_closure_fuzz.rs:107` (J09) | producto de capturas/union | pesado: producto de capturas/union; j09_effects::nested_owned_closure_drop_is_visible_in_outer_contract |
| 28 | 14 | `nexo::run_argv_matrix` | `generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle` | 48.2 (42.8/48.2/79.6) | 82.9 (82.9/58.4/138.4) | `tests/run_argv_matrix.rs:311` (A2/F0) | barrido: generated run cli argv matrix matches d5 d15 a2 oracle | fast: oráculo singular |
| 29 | 44 | `nexo` | `ir::i10a_tests::ownership_flow_limits_distinguish_local_and_compilation_scope` | 47.5 (49.0/47.5/42.8) | 10.5 (10.5/12.3/4.3) | `src/ir/i10a_tests.rs:201` (I10A) | ownership flow limits distinguish local and compilation scope | fast: testigo de cuota o diagnóstico |
| 30 | 37 | `nexo` | `frontend::migrated_parser_final::b_first_state_is_independent_of_the_offending_token` | 41.4 (35.9/41.4/48.4) | 13.7 (13.7/16.1/10.6) | `src/frontend/migrated_parser_final.rs:1381` (D15/frontend) | b first state is independent of the offending token | fast: caso normativo o sin sustituto corto acreditado |
| 31 | — | `nexo` | `ir::i10a_tests::verifier_and_effect_work_scale_with_branch_count` | 39.2 (40.9/39.2/36.0) | 5.0 (5.0/5.8/4.3) | `src/ir/i10a_tests.rs:865` (I10A) | barrido de trabajo de verificador/efectos | pesado: barrido de trabajo de verificador/efectos; effect_fixpoint_charges_each_operation_and_callsite_to_global_limit |
| 32 | 25 | `nexo` | `ir::i07c_tests::structured_deferred_check_and_run_match_live_twins` | 38.8 (38.8/39.8/35.0) | 38.9 (42.4/38.9/2.1) | `src/ir/i07c_tests.rs:6563` (I07C) | structured deferred check and run match live twins | fast: caso normativo o sin sustituto corto acreditado |
| 33 | 26 | `nexo` | `ir::i07c_tests::generated_monomorphic_constraints_match_independent_graph_oracle` | 34.2 (34.2/35.4/32.2) | 34.6 (36.6/34.6/2.8) | `src/ir/i07c_tests.rs:6169` (I07C) | grafo monomórfico generado | pesado: grafo monomórfico generado; statement_permutations_match_independent_constraint_oracle |
| 34 | 28 | `nexo::j05_match_fuzz` | `nested_guarded_match_resource_and_copy_sweep` | 33.6 (35.0/33.6/32.6) | 28.2 (28.2/28.7/7.8) | `tests/j05_match_fuzz.rs:112` (J05) | barrido de guardas | pesado: barrido de guardas; j05_match_guards::guard_fallthrough_and_body_states_join_after_selection |
| 35 | 33 | `nexo::mock_oracle` | `every_unicode_code_unit_escape_has_the_contract_rejection_phase_in_keys_and_values` | 31.7 (31.7/31.1/32.1) | 17.3 (21.9/17.3/4.1) | `tests/mock_oracle.rs:2826` (A2/F0) | every unicode code unit escape has the contract rejection phase in keys and values | fast: caso normativo o sin sustituto corto acreditado |
| 36 | 49 | `nexo` | `ir::i10a_tests::rejected_local_ownership_charges_do_not_consume_global_allowance` | 30.7 (33.2/30.7/26.8) | 8.1 (8.1/9.3/8.0) | `src/ir/i10a_tests.rs:289` (I10A) | rejected local ownership charges do not consume global allowance | fast: caso normativo o sin sustituto corto acreditado |
| 37 | 31 | `nexo::j09_closure_fuzz` | `capture_modes_wrappers_resources_and_calls_cross_product` | 28.4 (28.4/27.8/30.5) | 24.3 (24.3/25.9/1.9) | `tests/j09_closure_fuzz.rs:76` (J09) | capture modes wrappers resources and calls cross product | fast: caso normativo o sin sustituto corto acreditado |
| 38 | 29 | `nexo` | `frontend::tests::parser_small_stack_matrix_retains_nested_b_forms` | 28.0 (22.7/28.0/51.4) | 25.3 (21.3/25.3/269.3) | `src/frontend/tests.rs:4483` (D15/frontend) | sonda de pila: parser small stack matrix retains nested b forms | fast: sonda/frontera de pila o profundidad |
| 39 | 41 | `nexo::runtime_profiles_and_resource_limits` | `normal_type_solver_limit_is_reached_by_repeated_deep_array_comparisons` | 27.9 (28.1/27.9/19.9) | 12.7 (20.0/12.7/4.1) | `tests/runtime_profiles_and_resource_limits.rs:1271` (A2/F0) | normal type solver limit is reached by repeated deep array comparisons | fast: testigo de cuota o diagnóstico |
| 40 | 30 | `nexo::j04_match_fuzz` | `nested_option_match_resource_and_mode_sweep` | 25.2 (25.1/25.2/33.2) | 24.7 (29.7/24.7/3.4) | `tests/j04_match_fuzz.rs:115` (J04) | barrido de match Option | pesado: barrido de match Option; j04_match_semantics::temporary_scrutinee_uses_its_resolved_nominal_type_and_plan |
| 41 | 24 | `nexo::runtime_profiles_and_resource_limits` | `entry_tick_exhaustion_closes_every_temporary_resource_primitive` | 24.4 (24.4/24.0/68.6) | 47.9 (47.9/10.1/114.5) | `tests/runtime_profiles_and_resource_limits.rs:1575` (A2/F0) | entry tick exhaustion closes every temporary resource primitive | fast: caso normativo o sin sustituto corto acreditado |
| 42 | — | `nexo` | `frontend::migrated_parser_final::b_hostile_chains_return_diagnostics_on_a_small_stack` | 23.6 (19.7/23.6/48.4) | 4.5 (4.5/5.5/3.5) | `src/frontend/migrated_parser_final.rs:1578` (D15/frontend) | sonda de pila: b hostile chains return diagnostics on a small stack | fast: sonda/frontera de pila o profundidad |
| 43 | — | `nexo` | `ir::i07c_tests::source_array_depth_reports_the_complete_excessive_expression` | 20.5 (20.5/20.8/19.4) | 3.4 (3.4/3.8/1.8) | `src/ir/i07c_tests.rs:5487` (I07C) | source array depth reports the complete excessive expression | fast: sonda/frontera de pila o profundidad |
| 44 | 35 | `nexo` | `ir::f5_operator_tests::integer_results_match_separate_arbitrary_precision_oracle` | 20.4 (15.0/20.4/41.6) | 15.6 (15.6/19.7/6.8) | `src/ir/f5_operator_tests.rs:807` (IR general) | integer results match separate arbitrary precision oracle | fast: oráculo singular |
| 45 | 32 | `nexo::i07c_rd_conformidad` | `rd_table_matches_check_and_run` | 18.0 (18.0/17.3/28.6) | 21.7 (21.7/19.2/257.7) | `tests/i07c_rd_conformidad.rs:4` (I07C) | rd table matches check and run | fast: oráculo singular |
| 46 | — | `nexo` | `ir::tests::indexed_array_reads_borrow_selected_subtree_and_owner_context_copies` | 17.9 (18.9/16.8/17.9) | 4.8 (4.8/6.8/2.3) | `src/ir/tests.rs:1986` (IR general) | indexed array reads borrow selected subtree and owner context copies | fast: caso normativo o sin sustituto corto acreditado |
| 47 | 38 | `nexo` | `check::tests::generated_valid_mutations_return_valid_diagnostics_without_panics` | 17.6 (17.6/21.8/15.2) | 13.1 (13.1/16.6/11.2) | `src/check/tests.rs:3015` (checker) | barrido: generated valid mutations return valid diagnostics without panics | fast: caso normativo o sin sustituto corto acreditado |
| 48 | — | `nexo` | `ir::i10a_tests::ownership_limit_keeps_earlier_and_later_diagnostics_in_source_order` | 16.0 (16.5/16.0/14.7) | 4.5 (4.0/4.5/6.4) | `src/ir/i10a_tests.rs:387` (I10A) | ownership limit keeps earlier and later diagnostics in source order | fast: testigo de cuota o diagnóstico |
| 49 | 57 | `nexo::i08` | `result_array_quota_uses_expanded_source_and_releases_payload` | 15.9 (15.9/15.3/18.0) | 6.0 (7.3/6.0/4.2) | `tests/i08.rs:301` (I08) | result array quota uses expanded source and releases payload | fast: testigo de cuota o diagnóstico |
| 50 | 47 | `nexo::analysis_profiles_and_quotas` | `selected_profile_is_preserved_on_input_and_mock_environment_failures` | 15.6 (15.6/14.7/16.0) | 8.7 (8.0/8.7/83.9) | `tests/analysis_profiles_and_quotas.rs:392` (A2/F0) | selected profile is preserved on input and mock environment failures | fast: caso normativo o sin sustituto corto acreditado |
| 51 | — | `nexo` | `ir::i07c_tests::inferred_array_depth_has_a_deterministic_analysis_limit` | 15.3 (15.3/15.8/14.4) | 2.7 (2.4/2.7/3.4) | `src/ir/i07c_tests.rs:2866` (I07C) | inferred array depth has a deterministic analysis limit | fast: sonda/frontera de pila o profundidad |
| 52 | 50 | `nexo::runtime_profiles_and_resource_limits` | `normal_profile_work_frames_cells_and_observations_reach_all_three_boundary_points` | 13.4 (13.4/13.1/57.6) | 7.6 (4.8/7.6/103.2) | `tests/runtime_profiles_and_resource_limits.rs:470` (A2/F0) | normal profile work frames cells and observations reach all three boundary points | fast: caso normativo o sin sustituto corto acreditado |
| 53 | 40 | `nexo` | `ir::tests::malformed_source_campaign_never_panics_and_all_diagnostics_validate` | 13.3 (14.7/12.6/13.3) | 12.9 (19.3/12.9/0.9) | `src/ir/tests.rs:3751` (IR general) | malformed source campaign never panics and all diagnostics validate | fast: caso normativo o sin sustituto corto acreditado |
| 54 | 39 | `nexo::j09_closure_fuzz` | `captured_closure_fields_preserve_call_and_consumption_permissions` | 13.2 (13.2/12.9/19.9) | 13.0 (24.3/13.0/0.8) | `tests/j09_closure_fuzz.rs:136` (J09) | captured closure fields preserve call and consumption permissions | fast: caso normativo o sin sustituto corto acreditado |
| 55 | 42 | `nexo` | `frontend::tests::omitted_token_neighbor_matrix` | 12.5 (12.5/16.1/10.9) | 12.0 (12.0/14.9/9.0) | `src/frontend/tests.rs:414` (D15/frontend) | omitted token neighbor matrix | fast: caso normativo o sin sustituto corto acreditado |
| 56 | — | `nexo` | `interp::machine::tests::repeated_borrowed_index_releases_consumed_index_cells` | 11.0 (11.0/13.4/9.7) | 2.4 (2.4/3.2/1.8) | `src/interp/machine.rs:4237` (A2/F0) | repeated borrowed index releases consumed index cells | fast: caso normativo o sin sustituto corto acreditado |
| 57 | — | `nexo` | `interp::machine::tests::repeated_index_value_releases_unselected_array_cells` | 10.5 (10.5/12.9/9.4) | 2.5 (2.5/3.0/1.8) | `src/interp/machine.rs:4247` (A2/F0) | repeated index value releases unselected array cells | fast: caso normativo o sin sustituto corto acreditado |
| 58 | 45 | `nexo::j09_closure_fuzz` | `closure_capture_mode_union_and_nesting_sweep` | 10.3 (10.3/10.0/12.4) | 10.1 (10.3/10.1/0.8) | `tests/j09_closure_fuzz.rs:102` (J09) | closure capture mode union and nesting sweep | fast: caso normativo o sin sustituto corto acreditado |
| 59 | 54 | `nexo::j10_j13_fuzz` | `match_place_nonlocal_edges_across_loops_generators_and_closures` | 10.1 (10.1/10.1/17.4) | 6.7 (17.6/6.7/1.0) | `tests/j10_j13_fuzz.rs:127` (J10) | match place nonlocal edges across loops generators and closures | fast: caso normativo o sin sustituto corto acreditado |
| 60 | 48 | `nexo::j14_frontend_stack` | `measured_frontend_stack_minima_hold_for_valid_families` | 10.0 (10.0/9.9/69.6) | 8.4 (8.4/5.1/127.7) | `tests/j14_frontend_stack.rs:134` (J14) | sonda de pila: measured frontend stack minima hold for valid families | fast: sonda/frontera de pila o profundidad |
| — | 11 | `nexo` | `check::tests::semantic_passes_scale_with_source_bytes` | — | 116.0 (116.0/119.0/47.1) | `src/check/tests.rs:3240` (checker) | escala: semantic passes scale with source bytes | fast: caso normativo o sin sustituto corto acreditado |
| — | 13 | `nexo` | `frontend::parse::tests::parser_token_patterns_scale_with_source_length` | — | 84.4 (84.4/89.7/36.6) | `src/frontend/parse.rs:1042` (D15/frontend) | escala: parser token patterns scale with source length | fast: caso normativo o sin sustituto corto acreditado |
| — | 36 | `nexo::j14_accounting` | `published_new_operation_work_totals` | 2.6 (2.5/2.6/4.6) | 15.1 (15.1/2.5/61.3) | `tests/j14_accounting.rs:32` (J14) | published new operation work totals | fast: caso normativo o sin sustituto corto acreditado |
| — | 46 | `nexo` | `check::tests::semantic_passes_use_bounded_wide_stack_for_nested_forms` | 8.7 (8.7/10.2/7.3) | 9.6 (7.6/9.6/11.1) | `src/check/tests.rs:3105` (checker) | sonda de pila: semantic passes use bounded wide stack for nested forms | fast: sonda/frontera de pila o profundidad |
| — | 52 | `nexo` | `frontend::tests::formatter_preserves_ast_under_inserted_line_comments` | 6.2 (6.2/7.3/5.2) | 7.0 (5.6/7.0/9.3) | `src/frontend/tests.rs:3028` (D15/frontend) | formatter preserves ast under inserted line comments | fast: caso normativo o sin sustituto corto acreditado |
| — | 53 | `nexo` | `frontend::tests::multi_token_construct_comment_gaps_preserve_execution` | 5.8 (5.8/6.8/4.7) | 6.7 (5.3/6.7/9.2) | `src/frontend/tests.rs:1315` (D15/frontend) | multi token construct comment gaps preserve execution | fast: caso normativo o sin sustituto corto acreditado |
| — | 55 | `nexo::corpus` | `corpus_cli_check_run_diagnostics_match_across_profiles` | 4.1 (4.2/3.6/4.1) | 6.5 (6.5/4.8/25.8) | `tests/corpus.rs:91` (A2/F0) | corpus cli check run diagnostics match across profiles | fast: caso normativo o sin sustituto corto acreditado |
| — | 56 | `nexo::j14_s4_depth` | `generator_parameter_slot_and_late_declaration_charge_original_constructor` | 8.2 (8.2/7.4/58.1) | 6.4 (6.4/4.5/52.9) | `tests/j14_s4_depth.rs:131` (J14) | generator parameter slot and late declaration charge original constructor | fast: caso normativo o sin sustituto corto acreditado |
| — | 58 | `nexo::runtime_profiles_and_resource_limits` | `profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids` | 3.4 (3.5/3.4/3.1) | 5.4 (3.1/5.4/111.3) | `tests/runtime_profiles_and_resource_limits.rs:319` (A2/F0) | profile selection matrix covers defaults all profiles and unknown ids | fast: caso normativo o sin sustituto corto acreditado |
| — | 59 | `nexo::corpus` | `truncated_and_mutated_inputs_never_panic` | 4.5 (4.5/4.3/4.8) | 5.2 (7.5/5.2/0.3) | `tests/corpus.rs:574` (A2/F0) | barrido: truncated and mutated inputs never panic | fast: caso normativo o sin sustituto corto acreditado |
| — | 60 | `nexo::j14_s4_depth` | `option_depth_reports_once_at_the_first_exceeding_constructor` | 7.3 (7.3/6.3/32.8) | 5.1 (5.1/3.9/45.6) | `tests/j14_s4_depth.rs:410` (J14) | option depth reports once at the first exceeding constructor | fast: sonda/frontera de pila o profundidad |

## Tabla de pesados para el filtro

<!-- NX-BUILD-EXCLUSIONS-START -->
| binary_id | test_name | mediana_debug_s | mediana_release_s | motivo y cobertura corta | medición |
| --- | --- | ---: | ---: | --- | --- |
| `nexo` | `ir::i07a_tests::cfg_local_branch_scale_uses_sparse_temp_counters` | 290.2 | 11.4 | barrido de CFG 64/256 KiB; if_values_after_other_operands_remain_available_across_cfg_edges | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07a_tests::later_operand_dominance_verification_scales_by_cfg_work` | 162.5 | 7.0 | barrido de dominancia 64/128 KiB; if_values_after_other_operands_remain_available_across_cfg_edges | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07a_tests::repeated_while_preheaders_compile_at_two_source_sizes` | 74.7 | 2.5 | barrido de preheaders; while_backedge_rechecks_complete_branching_condition | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07a_tests::directed_branch_error_and_divergence_fuzz_has_no_internal_failure` | 138.8 | 128.0 | fuzz dirigido de ramas; directed_divergence_positions_match_reference_acceptance_and_diagnostics | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07a_tests::mutated_cfg_sources_keep_rejections_structured` | 81.7 | 74.5 | campaña de CFG mutado; cfg_verifier_rejects_bad_edges_branch_type_and_missing_join_definition | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07b_tests::consumed_prefix_then_loop_suffix_converges_before_each_exit` | 224.9 | 71.1 | barrido de 700–3000 consumos y bucles; loop_exit_lists_all_reaching_consumptions_and_backedge | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07b_tests::repeated_loop_analysis_has_linear_step_growth` | 73.8 | 2.5 | barrido de pasos de bucle; while_condition_effects_reach_body_and_false_exit | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07b_tests::consumed_bindings_before_many_small_loops_keep_sparse_state_linear` | 60.5 | 4.2 | barrido de estado disperso; loop_exit_lists_all_reaching_consumptions_and_backedge | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07b_tests::generated_nested_loop_ownership_matches_source_ast_model` | 127.6 | 121.3 | modelo generado de propiedad en bucles; condition_arm_effects_cover_values_branches_and_use_positions | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07b_tests::generated_sparse_ownership_matches_dense_reference_in_regular_tests` | 127.8 | 121.5 | modelo generado disperso/denso; shared_node_nested_condition_matches_dense_reference | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i10a_tests::verifier_and_effect_work_scale_with_branch_count` | 39.2 | 5.0 | barrido de trabajo de verificador/efectos; effect_fixpoint_charges_each_operation_and_callsite_to_global_limit | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::tests::generated_scalar_programs_match_independent_decimal_model` | 186.0 | 215.4 | 8.000 programas aritméticos; f5_operator_tests::integer_results_match_separate_arbitrary_precision_oracle | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::tests::generated_array_copies_and_writes_match_independent_matrix_model` | 185.9 | 215.4 | 8.000 matrices de arrays; ordinary_array_parameter_returns_deep_copy y selected_array_copy_is_independent_of_mutable_root | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::tests::array_sources_with_mutated_syntax_never_panic_and_validate_rejections` | 117.6 | 126.1 | campaña de arrays mutados; heterogeneous_array_rejection_has_homogeneous_positive y malformed_source_campaign_never_panics_and_all_diagnostics_validate | 2026-11-19 / `ae9130f` código |
| `nexo` | `frontend::tests::canonical_printer_preserves_full_b_ast_across_body_and_signature_matrix` | 130.1 | 132.7 | matriz B de impresor; accepted_b_ast_round_trips_without_source_spans | 2026-11-19 / `ae9130f` código |
| `nexo` | `frontend::tests::lexer_and_parser_scale_linearly_across_surface_families` | 123.7 | 32.0 | barrido léxico y parser; published_syntax_rows_have_exact_tokens_spans_and_prefixes | 2026-11-19 / `ae9130f` código |
| `nexo` | `check::tests::valid_generated_names_shadowing_and_comparisons_have_no_diagnostics` | 115.6 | 106.7 | fuentes generadas de nombres; duplicate_binding_and_inner_shadowing_are_lexical y comparisons_use_right_operand_for_mismatch_and_operator_for_domain | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::effects::witness_differential_tests::dynamic_witness_matches_reference_with_variable_leaf_chains_and_sccs` | 86.9 | 77.2 | cadenas/SCC de testigos; dynamic_witness_matches_simple_path_enumerator_on_small_graphs | 2026-11-19 / `ae9130f` código |
| `nexo` | `ir::i07c_tests::generated_monomorphic_constraints_match_independent_graph_oracle` | 34.2 | 34.6 | grafo monomórfico generado; statement_permutations_match_independent_constraint_oracle | 2026-11-19 / `ae9130f` código |
| `nexo::j04_match_fuzz` | `nested_option_match_resource_and_mode_sweep` | 25.2 | 24.7 | barrido de match Option; j04_match_semantics::temporary_scrutinee_uses_its_resolved_nominal_type_and_plan | 2026-11-19 / `ae9130f` código |
| `nexo::j05_match_fuzz` | `nested_guarded_match_resource_and_copy_sweep` | 33.6 | 28.2 | barrido de guardas; j05_match_guards::guard_fallthrough_and_body_states_join_after_selection | 2026-11-19 / `ae9130f` código |
| `nexo::j09_closure_fuzz` | `nested_capture_permissions_and_union_carriers_cross_product` | 53.4 | 51.7 | producto de capturas/union; j09_effects::nested_owned_closure_drop_is_visible_in_outer_contract | 2026-11-19 / `ae9130f` código |
<!-- NX-BUILD-EXCLUSIONS-END -->

La tabla usa valores literales únicos. Para actualizarla: medir tres corridas debug/release, leer los flujos normativos y los casos cortos, registrar el par exacto, repetir `cargo nextest list` en ambos perfiles y ejecutar `python -B scripts/check-nextest-selection.py`. Un test nuevo entra en fast por defecto. Los excluidos se ejecutan con resultado terminal en `--profile default`.

## Equivalencia cargo test / nextest en la base

En target conservado, `cargo test --locked --offline -- --list` y `cargo test --locked --offline -- --list --ignored` (también con `--release`) dieron el mismo conjunto descubierto por par `(binary_id, test_name)`. Las corridas completas de `cargo test` y nextest dieron los mismos resultados por par: debug 1.979 `ok` + 72 `ignored`; release 1.981 `ok` + 73 `ignored`; 0 fallos y 0 doctests. Pared cargo test debug 978,9 s, release 824,0 s. La salida mezclada de `--list --ignored` en PowerShell intercala algunos encabezados por buffering; los nombres ignorados se compararon como multiconjunto y el estado por par se confirmó en la corrida real de `cargo test`.

## Variantes de construcción condicionadas

No se ejecutaron ensayos de `CARGO_BUILD_JOBS=2/6`, enlace `rust-lld`, `debug = "line-tables-only"` ni consolidación de binarios: antes de esos ensayos se exige la demostración de equivalencia de máximos y margen de pila de todas las sondas J16 a 204/256 KiB en proceso separado. Esa demostración no está disponible en esta campaña. Los tiempos de `cargo test --no-run` sin instrumentación tampoco separan compilación de enlace, por lo que no se atribuye una ganancia al enlazador. `CARGO_BUILD_JOBS=12` queda NO EVALUADO por el límite expreso de seis workers. No se adopta ningún ajuste de jobs, linker, depuración o consolidación y no se modifican `Cargo.toml` ni `.cargo/config.toml`.

## Puertas frías

Cada muestra parte de `git archive` y target vacío bajo `C:/nexo-work/tmp/nx-build-*`, con seis workers y `--locked --offline`; fmt, Clippy `-D warnings`, nextest debug y release se cronometran por separado. Se eliminan copia y target tras exit 0.

| Estado | r1/r2/r3 total (s) | mediana | mínimo/máximo | fmt | Clippy | selección | debug | release |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- |
| before | 1378.5 / 1476.5 / 1558.0 | 1476.5 | 1378.5–1558.0 | 3.6 / 3.8 / 31.7 | 33.8 / 30.8 / 32.2 | — | 701.8 / 848.5 / 705.1 | 639.3 / 593.4 / 789.1 |
| after | 1081.7 / 1234.2 / 900.4 | 1081.7 | 900.4–1234.2 | 2.0 / 2.9 / 3.6 | 15.0 / 21.1 / 27.6 | — | 462.5 / 459.5 / 570.3 | 602.2 / 750.7 / 298.9 |
| fast-before | 1200.4 / 1337.3 / 1202.2 | 1202.2 | 1200.4–1337.3 | 5.9 / 2.6 / 4.4 | 51.4 / 30.4 / 22.5 | 551.2 / 645.8 / 689.1 | 398.1 / 422.7 / 366.1 | 193.8 / 235.6 / 120.1 |
| fast-after | 1140.3 / 669.5 / 1010.5 | 1010.5 | 669.5–1140.3 | 3.8 / 5.1 / 2.2 | 25.9 / 34.9 / 17.4 | 656.8 / 304.7 / 472.1 | 327.2 / 228.2 / 371.3 | 126.6 / 96.6 / 147.5 |

## Fast con target conservado

Después de cada fast frío se repitió `sh scripts/validate-fast.sh` sin borrar el target, manteniendo el mismo perfil y chequeo de selección.

| Estado | r1/r2/r3 (s) | Mediana | Mínimo/máximo |
| --- | --- | ---: | --- |
| fast-before | 620.7 / 653.9 / 545.1 | 620.7 | 545.1–653.9 |
| fast-after | 471.4 / 509.5 / 575.9 | 509.5 | 471.4–575.9 |

Las tres réplicas fast-after (fría y caliente) terminaron con exit 0. Se descarta aparte una repetición caliente de la segunda copia anterior al timeout: un hijo `nexo.exe check` quedó sin CPU durante >717 s, pero el comando y test aislados pasaron y el problema no volvió en las repeticiones. No se incluye su reloj de 788,7 s en ninguna mediana.

## Puerta final y diferencial

En un clon limpio de `457ade7` con los archivos de NX-B1 superpuestos, `NEXTEST_PROFILE=fast sh scripts/validate.sh` terminó exit 0 en 605,5 s: fmt, Clippy `-D warnings`, 1.979/1.979 ejecutables debug y 1.981/1.981 release. El perfil del entorno no redujo la puerta completa. `python -B scripts/check-nextest-selection.py` dio `completo=1979, fast=1957, pesados=22, ignorados=72` y `completo=1981, fast=1959, pesados=22, ignorados=73`. Un control negativo temporal que renombró un test de la tabla a `__nx_no_such_test__` dio exit 1 por par ausente; ambos archivos se restauraron por SHA-256. `python -B scripts/objetivo.py` informó 15/29. La verificación RD informó 152/152 pares `check`/`run`, cero diferencias. Las 58 invocaciones `check`/`run` de los 29 programas objetivo, frente al binario base, coincidieron byte a byte en exit, stdout y stderr.

## Reloj por binario

Una corrida adicional de `libtest-json` registró la llegada de los eventos `started` y `ok` de cada par. La pared aproximada de un binario es desde su primer inicio hasta su último final; la suma de tests del mismo binario se muestra aparte y puede ser mayor por paralelismo.

| binary_id | pared debug (s) | suma tests debug (s) | pared release (s) | suma tests release (s) |
| --- | ---: | ---: | ---: | ---: |
| `nexo` | 354.5 | 4275.7 | 216.1 | 1524.3 |
| `nexo::analysis_profiles_and_quotas` | 0.0 | 67.9 | 0.0 | 25.7 |
| `nexo::bin/nexo` | 0.0 | 0.0 | 0.0 | 0.5 |
| `nexo::canonical_print` | 0.0 | 1.0 | 0.0 | 1.1 |
| `nexo::cli` | 0.0 | 31.3 | 0.0 | 40.9 |
| `nexo::cli_argument_and_unicode_edges` | 0.0 | 2.2 | 0.0 | 4.8 |
| `nexo::corpus` | 0.0 | 11.1 | 0.0 | 14.3 |
| `nexo::corpus_objetivo` | 0.0 | 0.0 | 0.0 | 0.1 |
| `nexo::explain_and_print_output` | 0.0 | 0.8 | 0.0 | 1.2 |
| `nexo::f0007` | 0.0 | 5.3 | 0.0 | 7.1 |
| `nexo::i01_diagnostic` | 0.0 | 2.9 | 0.0 | 4.2 |
| `nexo::i07c_causal_obligations` | 0.0 | 7.4 | 0.0 | 12.2 |
| `nexo::i07c_deferred_group_regressions` | 0.0 | 0.6 | 0.0 | 0.9 |
| `nexo::i07c_deferred_mutation_cases` | 0.0 | 1.6 | 0.0 | 2.7 |
| `nexo::i07c_deferred_twins` | 0.0 | 1.5 | 0.0 | 2.4 |
| `nexo::i07c_mutation_depth` | 0.0 | 0.3 | 0.0 | 0.4 |
| `nexo::i07c_mutation_regressions` | 0.0 | 4.6 | 0.0 | 6.7 |
| `nexo::i07c_normative_survivors` | 0.0 | 22.2 | 0.0 | 2.7 |
| `nexo::i07c_pending_nominal_domain` | 0.0 | 2.9 | 0.0 | 3.7 |
| `nexo::i07c_rd_conformidad` | 0.0 | 36.9 | 0.0 | 154.0 |
| `nexo::i08` | 0.0 | 160.2 | 0.0 | 19.0 |
| `nexo::i08_robustness` | 0.0 | 0.4 | 0.0 | 0.7 |
| `nexo::i11a` | 314.9 | 5.8 | 188.3 | 4.8 |
| `nexo::j01_profile` | 0.0 | 2.7 | 0.0 | 4.9 |
| `nexo::j02_option` | 0.0 | 0.4 | 0.0 | 0.6 |
| `nexo::j03_match` | 0.0 | 1.1 | 0.0 | 2.6 |
| `nexo::j04_match_fuzz` | 0.0 | 22.0 | 0.0 | 10.0 |
| `nexo::j04_match_ownership` | 0.0 | 1.3 | 0.0 | 88.4 |
| `nexo::j04_match_semantics` | 0.0 | 1.8 | 0.0 | 12.7 |
| `nexo::j05_match_fuzz` | 0.0 | 23.9 | 0.0 | 1.9 |
| `nexo::j05_match_guards` | 0.0 | 2.4 | 0.0 | 21.7 |
| `nexo::j06_j09_validation` | 0.0 | 2.2 | 0.0 | 5.7 |
| `nexo::j06_names` | 0.0 | 0.8 | 0.0 | 3.7 |
| `nexo::j07_retention` | 0.0 | 0.9 | 0.0 | 5.7 |
| `nexo::j08_receiver` | 0.0 | 0.7 | 0.0 | 6.1 |
| `nexo::j09_closure_fuzz` | 292.7 | 76.4 | 173.2 | 7.5 |
| `nexo::j09_effects` | 0.0 | 0.7 | 0.0 | 9.6 |
| `nexo::j10_generator` | 0.0 | 0.5 | 0.0 | 5.0 |
| `nexo::j10_j13_fuzz` | 311.2 | 11.6 | 172.7 | 1.2 |
| `nexo::j10_j13_regressions` | 0.0 | 0.6 | 0.0 | 3.5 |
| `nexo::j11_generator` | 0.0 | 0.8 | 0.0 | 3.5 |
| `nexo::j12_generator` | 0.0 | 0.7 | 0.0 | 2.2 |
| `nexo::j13_iteration` | 0.0 | 2.0 | 0.0 | 6.2 |
| `nexo::j14_accounting` | 0.0 | 3.7 | 0.0 | 14.5 |
| `nexo::j14_diagnostic_publication` | 0.0 | 0.7 | 0.0 | 1.7 |
| `nexo::j14_dispatch_guard` | 0.0 | 4.9 | 0.0 | 1.3 |
| `nexo::j14_edge_cleanup` | 0.0 | 4.9 | 0.0 | 10.3 |
| `nexo::j14_frontend_stack` | 0.0 | 9.3 | 0.0 | 30.4 |
| `nexo::j14_match_inference_edges` | 0.0 | 3.2 | 0.0 | 11.8 |
| `nexo::j14_s3_values` | 0.0 | 10.2 | 0.0 | 16.3 |
| `nexo::j14_s4_depth` | 0.0 | 130.8 | 0.0 | 85.8 |
| `nexo::j14_semantic_boundaries` | 0.0 | 19.6 | 0.0 | 81.4 |
| `nexo::j15_cli` | 0.0 | 5.5 | 0.0 | 39.9 |
| `nexo::j16_diagnostics` | 0.0 | 3.3 | 0.0 | 7.2 |
| `nexo::j16_for_scope` | 0.0 | 0.1 | 0.0 | 0.5 |
| `nexo::j16_frontend_stack` | 0.0 | 10.9 | 0.0 | 37.5 |
| `nexo::j16_integrated` | 0.0 | 28.7 | 0.0 | 56.1 |
| `nexo::j16_question_edge` | 0.0 | 0.5 | 0.0 | 1.2 |
| `nexo::mock_env_json_and_read_boundaries` | 0.0 | 36.5 | 0.0 | 38.7 |
| `nexo::mock_env_records_and_failures` | 0.0 | 0.9 | 0.0 | 4.2 |
| `nexo::mock_oracle` | 292.4 | 145.1 | 159.5 | 28.0 |
| `nexo::parenthesized_places` | 0.0 | 0.3 | 0.0 | 5.2 |
| `nexo::run_argv_dash_value_semantics` | 0.0 | 0.7 | 0.0 | 35.4 |
| `nexo::run_argv_matrix` | 0.0 | 54.0 | 0.0 | 160.2 |
| `nexo::run_cli_diagnostics_and_cells` | 0.0 | 7.6 | 0.0 | 56.3 |
| `nexo::runtime_profiles_and_resource_limits` | 263.1 | 168.8 | 92.7 | 287.0 |

## NX-B2 — decisión P1 y controles P2 (2026-11-19)

**P1 no adoptado.** El criterio N1 exige cuatro condiciones simultáneas y, antes de modificar `Cargo.toml`, el plan exige máximos instrumentados de pila por fase en las sondas de 204/256 KiB, incluido el alias `measured_stack_child`, sin aumento observable. La base actual solo expone sondas de límite (`NEXO_PROBE_*_KIB` en `src/lib.rs`), no un medidor de máximos. La medición de ese máximo no está disponible sin instrumentar el núcleo, que queda fuera del alcance de C. Por ello B de N1 (equivalencia de pila medida) permanece **NO EVALUADO** y P1 no se integra. La muestra de A sobre copia sí observó 0 tests originales ausentes, un alias adicional, 72/73 ignorados, −39,1 % de mediana de compilación fría debug y −72,1 % de bytes de target; no son una evaluación completa de A–D de N1 ni sustituyen las tres repeticiones de puerta completa antes/después. `Cargo.toml` y `tests/` permanecen intactos. Hay 14 fuentes con `mod common;`; la comprobación de colisión bajo `cargo test` queda pendiente para un nuevo intento de P1.

**P2.** En una crate sintética bajo `C:/nexo-work/tmp/`, con test que crea marcador y duerme 4 s en el primer intento y timeout de 1 s, `NEXTEST_RETRIES=1 sh scripts/validate.sh` y `sh scripts/validate-fast.sh` terminaron exit 100, `1 timed out`, cero reintentos. Con `NEXTEST_PROFILE=fast NEXTEST_RETRIES=1`, `validate.sh` informó perfil `default` y terminó exit 100. Se conservaron fmt, Clippy y fallback íntegro. La configuración propuesta eleva fast a 11 min: 660/414,0 = 1,59 respecto al mayor de las tres muestras previas del test no excluido (376,4/414,0/381,6 s); 354,5 s era la pared del binario `nexo`.
El mismo test no excluido, `ir::i10a_tests::ir_item_limit_is_deferred_behind_ownership_and_effect_errors`, pasó tres veces con `cargo nextest run --locked --offline --profile default --retries 0 -E 'test(=ir::i10a_tests::ir_item_limit_is_deferred_behind_ownership_and_effect_errors)'`: **274,223 / 227,656 / 207,989 s**, mediana 227,656 s, máximo 274,223 s. La máquina compartía 6–7 procesos Cargo y, en una lectura durante la serie, 27 `rustc`, 16 `link` y 3 `nexo`; la carga no fue estable. El máximo propio no exige subir 660 s (factor 2,41); el máximo anterior de 414,0 s sigue gobernando la decisión.

Puerta P2 en copia `git archive f414c7a` con los cinco archivos P2 superpuestos, target nuevo bajo `C:/nexo-work/tmp/`, Rust 1.99.1, seis workers: `sh scripts/validate.sh` exit 0 en **805,010 s** de pared; fmt exit 0; Clippy `-D warnings` exit 0 (19,87 s de Cargo), build debug 58,46 s, nextest debug 480,696 s (1.979/1.979, 72 ignorados), build release 145 s, nextest release 96,255 s (1.981/1.981, 73 ignorados). Objetivo 15/29 y RD 152/152, cero diferencias, exit 0. En la puerta se observaron hasta 8 procesos Cargo concurrentes de la máquina compartida. Esta es **una** réplica fría posterior; no se presenta como mediana de tres ni como comparación causal con la campaña NX-B1. Copia y target se eliminaron al terminar.

## NX-B2-P3 — veredictos históricos y parada temprana (2026-11-19)

Los parches exactos, hashes y perfiles están en `docs/validation/NX-build-mutant-verdicts.json`. Se buscaron commits con `git log --format=%H -- <archivo>` y se comprobó el SHA-256 de `git show <commit>:<archivo>` contra `sha_initial`; cada mutante se ejecutó en `git archive` de su commit bajo `C:`. `O0003` quedó descartado: ninguno de los 25 commits disponibles de `src/check/types.rs` reprodujo su prehash `07540dd0…b7d43fe7c`. No se aplicó a HEAD. Los commits reproducibles fueron `eb0a323` para `P-block-depth` y `eb7a72d` para `T0-group-shape`; ambos parches reprodujeron exactamente el `sha_mutated` archivado y se restauraron a `sha_initial`.

| Mutante | nextest default sin filtro, `--retries 0` | `cargo test` íntegro | Comparación |
| --- | --- | --- | --- |
| `P-block-depth` | MUERTO, debug exit 100, 50,780 s de pared; 295/1.898 tests iniciados, 1 fallo: `frontend::migrated_parser_final::b_structural_budgets_have_literal_boundaries_and_diagnostic_fields` | debug exit 101, 404,022 s; 1.127 pasados, 2 fallidos y 53 ignorados en el binario de biblioteca; el mismo test falló | mismo veredicto y test que mata; el oráculo clásico detuvo la suite tras el binario fallido |
| `T0-group-shape` | SUPERVIVIENTE, debug 481,699 s (1.896/1.896, 72 ignorados), release 427,254 s (1.898/1.898, 73 ignorados) | debug 982,207 s (1.896 pasados, 72 ignorados), release 584,916 s (1.898 pasados, 73 ignorados); ambos exit 0 | mismo veredicto; se ejecutaron los dos perfiles completos |

El test sintético `clock_probe` pasó con 10 ms de espera. Con el parche de 60 s y `slow-timeout = { period = "1s", terminate-after = 1 }`, nextest terminó exit 100 con `event=failed`, `reason="time limit exceeded"` y `1 timed out`; el helper lo clasificó INCONCLUSO, no MUERTO. Una primera sonda con límite de proceso de 2 s alcanzó el límite durante compilación y tampoco se contó como muerte. Los procesos iniciados se cerraron por su PID; no quedaron hijos de la sonda. La campaña I07c en curso conserva su clasificador y no se reconfiguró.

## NX-B2-P4 — registro de puerta y condición de caché

`scripts/gate.sh` archiva el commit y conserva el hash de árbol, los hashes de los cuatro archivos de puerta (`validate.sh`, `validate-fast.sh`, `check-nextest-selection.py`, `.config/nextest.toml`), el del propio script, las versiones de Rust y nextest, entorno relevante, log SHA-256, exits y recuentos por perfil. Solo publica GREEN si `validate.sh`, objetivo y RD salen 0 y los resúmenes terminales concuerdan con `nextest list` default (o con listados íntegros de `cargo test`). El verificador exige la misma identidad y un log intacto. `git archive` en Windows produjo CRLF en scripts cuyo blob Git tiene LF; la comprobación acepta únicamente el hash literal o el de normalizar CRLF a LF y registra el hash del archivo del commit.

Controles en repo sintético de un test, Rust 1.99.1 y nextest 0.9.152: árbol sano con nextest produjo GREEN y `--verify` lo reprodujo; el fallback `cargo test` también produjo GREEN con recuentos iguales. Un test roto produjo FAIL exit 100 bajo `NEXTEST_RETRIES=1`; un `validate.sh` sintético que devolvía 0 sin resumen produjo FAIL por resumen ausente; cambiar `NEXTEST_RETRIES` invalidó `--verify` por identidad distinta. Los registros de las dos variantes del mismo árbol tuvieron el mismo conjunto de tests (un test debug y release). Son controles de arnés, no evidencia de puerta del repositorio real.

Medición condicional de caché para diferenciales `4a61ee1`→`1ace709`: tres repeticiones con `git archive`, target nuevo por commit en `C:`, seis workers, orden alternado; se compiló `nexo.exe` debug/release y se comparó con una copia byte a byte de cada binario. En cada variante se ejecutaron los 29 programas objetivo con `check` y `run` en ambos perfiles (116 invocaciones por réplica) y se compararon exit, stdout y stderr, sin diferencias. La carga fue variable; en la tercera réplica se observaron hasta 49 `rustc` y 47 `link` de la máquina compartida.

| Réplica | recompilar + ejecutar (s) | copiar + ejecutar (s) | ahorro |
| --- | ---: | ---: | ---: |
| 1 | 131,339 | 6,997 | 94,67 % |
| 2 | 382,351 | 19,749 | 94,83 % |
| 3 | 273,753 | 17,571 | 93,58 % |
| mediana | **273,753** | **17,571** | **93,58 %** |

La medición supera el umbral del 20 % de P4 para el subpaso P4-cache; su integración exige un commit separado con manifiesto ligado al registro verde. No se usa ninguna copia para árboles mutados.

### NX-B2-P4-cache — publicación y lectura

El subpaso publica `debug.exe`, `release.exe` y un manifiesto atómico solo tras el registro GREEN de la puerta, bajo el hash de árbol y la versión de `rustc`. El manifiesto referencia la identidad y el SHA-256 del log verde y contiene el SHA-256 de ambos binarios. La lectura `--cached-bin` vuelve a verificar registro, log, identidad, manifiesto y binario; `NEXO_MUTANT=1` se rechaza. Se recortan únicamente entradas reconstruibles propias cuando la caché supera 4 GB.

En un repo sintético, tras construir ambos binarios, la puerta publicó GREEN y `--cached-bin debug` y `--cached-bin release` devolvieron binarios con SHA-256 válido. Al añadir un byte al binario debug de esa copia, `--cached-bin debug` terminó con exit 1 y `SHA-256 de binario en caché distinto`. La copia dañada no se usa como evidencia del repositorio Nexo. El diferencial real de tres repeticiones de la tabla anterior satisface el umbral de adopción de P4-cache con salidas byte a byte idénticas.
