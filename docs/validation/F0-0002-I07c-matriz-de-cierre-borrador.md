# F0-0002 I07c — Matriz de cierre (BORRADOR para los validadores B y D)

Estado del documento: borrador de trabajo. No declara PASS, ni «cerrado», ni aprueba el corte. Ordena la
evidencia existente regla por regla para que B y D la contrasten; cada fila debe poder refutarse leyendo
el test citado. Base: worktree `C:\projects\Nexo-i07c`, rama `i07c-estructural`, HEAD `0650fee`.
Autoría de la matriz: lectura y `grep` (no se compiló, no se ejecutó ninguna suite, no se tocó código ni ADR).

Fuentes leídas: ADR `docs/adr/F0-0002-second-slice.md` §4.1, §4.1.1, §4.1.2 (N(a)–N(j), I(a)–(c), cláusulas 1–7
y sus tablas de ejemplos), §14.10 (TD01–TD07, literales de fondo, tabla RD01–RD128), §14.11 (DG07/DG08 y DG04–DG06),
§11.8 «A. `type_solver_steps`» y filas de cuota de §11.1; ADR `docs/adr/F0-0003-third-slice.md` (párrafo
«Constructor de patrón sobre raíz monomórfica abierta» y «Ejemplos de raíz abierta», que I07c cierra);
contrato `docs/validation/F0-0002-I07c-contrato-de-cierre.md`; estado `docs/validation/F0-0002-I07c-estado.md`;
`Pending.md` (filas `I07C-*`); `docs/validation/i07c-mutations-ccf/RESULTADO.md` y
`docs/validation/i07c-mutations-s4/RESULTADO.md`.

## 0. Estados, abreviaturas y cómo leer la matriz

Estados (solo estos tres):

- **cubierta**: hay test o regresión con aserción sobre código, subcausa, primario/token y/o campos exigidos
  (o valor de cuota), con positivo pareado si es regla de rechazo.
- **cubierta solo por mutación**: ningún test dirigido la fija, pero mutantes detectados la rompen (se citan IDs).
- **sin cobertura**: no hay test ni mutante que la fije (o falta el positivo pareado, el literal exacto, etc.).

Abreviaturas de fichero (todas bajo `C:\projects\Nexo-i07c\`):

| Abrev. | Fichero |
|---|---|
| DC | `tests/i07c_deferred_constraints.rs` |
| PD | `tests/i07c_pattern_deferred.rs` |
| PN | `tests/i07c_pending_nominal_domain.rs` |
| CO | `tests/i07c_causal_obligations.rs` |
| MR | `tests/i07c_mutation_regressions.rs` |
| NS | `tests/i07c_normative_survivors.rs` |
| SD | `tests/i07c_solver_depth.rs` |
| DT | `tests/i07c_deferred_twins.rs` |
| DG | `tests/i07c_deferred_group_regressions.rs` |
| DM | `tests/i07c_deferred_metamorphic.rs` (ambas pruebas `#[ignore]`) |
| DMC | `tests/i07c_deferred_mutation_cases.rs` |
| MD | `tests/i07c_mutation_depth.rs` |
| S4 | `tests/i07c_s4_context_regressions.rs` |
| RD | `tests/i07c_rd_conformidad.rs` (llama a `scripts/i07c_rd_conformidad.py --verify`; fixture `tests/fixtures/i07c_rd.json`) |
| IT | `src/ir/i07c_tests.rs` (biblioteca, `#[cfg(test)]`) |
| I7A | `src/ir/i07a_tests.rs` |
| QT | `src/check/types.rs`, módulo `quota_tests` (y `i07c_survivor*` en el módulo de tests de ese fichero) |
| CT | `src/check/tests.rs` |

Todos los ficheros de `tests/i07c_*.rs` están registrados en `tests/suite.rs` (salvo `i07c_mutant_projection_scale.rs`,
que `scripts/check-nextest-selection.py` trata aparte y cuya prueba está `#[ignore]`). Lo no `#[ignore]` corre en
`sh scripts/validate.sh` en debug y release. Las pruebas `#[ignore]` (campañas) NO forman parte de esa puerta.

### 0.1 Cinco hechos sobre la evidencia que condicionan la lectura de todas las filas

1. **El test RD (`rd_table_matches_check_and_run`) es una red amplia pero floja.** Verifica, por cada una de las 152
   fuentes y para `check` y `run` por CLI: código de salida, aceptación/rechazo, que cada C esperado (código, subcausa,
   primario) esté presente, que no haya dos diagnósticos con el mismo primario, que todo diagnóstico tenga código en
   {NX-E0102, NX-E0103, NX-E1303} y primario no vacío dentro de la fuente. **No falla** por: un diagnóstico extra
   (otro código o primario distinto, p. ej. un C espurio o un V que serialice un tipo pendiente), por relaciones, por
   `data`, ni por presencia/ausencia de D/V (esas diferencias solo se imprimen como `SNAPSHOT`, informativas).
   Consecuencia: «ningún C espurio» y «no se serializa un tipo pendiente» NO quedan fijados por RD; hay que apoyarse en
   tests con `errors.len()==1` exacto (se citan fila a fila).
2. **TD, DG07/DG08 y la mayoría de tests dirigidos pasan por la API de biblioteca** (`compile_scalar`, `check_proposed`,
   `compile_scalar_with_profile`), no por la CLI. Salida 1, `outcome.kind=rejected` y «no se ejecutan primitivas»
   solo se afirman para RD (CLI), PD/DC en los tests con `run_entry` (compilación en modo `run`) y S4 (CLI).
3. **Algunos esperados no proceden del ADR.** `S4` (`contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli`)
   compara el JSON completo con salidas transcritas de `be8c47c` (base B1b), y los `survivor_m*_check_diagnostics`
   de SD (lote 6) tienen esperados «de las salidas de referencia» (`docs/correcciones/I07c-C-mutaciones-lote6.md`);
   RESULTADO.md de ccf reconoce que M00474, M00475 y M00655 no tienen ejemplo normativo en el ADR. Fijan la
   implementación, no el texto del ADR.
4. **Los tests `adr_gap_*` no son oráculo** (AGENTS.md y `Pending.md` fila `I07C-ADR-LAGUNAS`): se citan solo si
   además hay aserción independiente, y se marca en la observación.
5. **Offsets TD/DG.** El ADR transcribe TD/DG en la superficie previa a F0-D15 (`fn main`, `if true{…}`) y permite
   recalcular offsets (§14.10). Los tests usan `fn Main` y `(if true then {return;}else{return;})` (37 bytes; el
   `DV` de RD es de 39 bytes). Cuando el test usa DV, los offsets difieren de los literales del ADR por esa
   recomputación y se comparan con el recomputado; no se verificó byte a byte contra una tabla del ADR.

## 1. §4.1 (base de inferencia, párrafos 4 y 5)

| ID | Regla (cita) | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| B-1 | §4.1: «`Ok(1)` necesita determinar también el tipo de error. `Err(7)` … Si el contexto local no lo resuelve, se emite `NX-E0113`.» | `NX-E0113`, `types.infer`; `unresolved_components` `$.err` / `$.ok` | `tests/f0007.rs::result_constructor_posterior_context_reports_adr_negative_diagnostics` (literales `Ok(1)` y `Err(7)`); `tests/corpus.rs::result_expression_forms_use_semantic_rules` (`Ok(1)` `[16,21)`, `Err(1)` `[16,22)`); positivos `tests/f0007.rs::result_constructor_negative_repairs_and_control_contexts` | cubierta | Los esperados vienen de la tabla de F0-0007, no de F0-0002; el literal `Err(7)` está en f0007, el de corpus usa `Err(1)`. |
| B-2 | §4.1: «Un array vacío o constructor Array conserva `NX-E0103` si su elemento sigue sin resolver al cierre de la función.» | `NX-E0103`, `subcause=null`, `types.infer`, primario el constructor | `tests/i01_diagnostic.rs::ordinary_explanations_match_literal_published_pairs_and_fixture_profile` (par `fn Main(){let xs=[];}` / anotado); PD `pattern_local_exports_closed_array_element` (`match Some([])…` primario `[21,23)` con todos los campos y `run_entry` false/true); RD101, RD122, RD127 (ver §6) | cubierta | |

(§4.1 «Result<T,E> copiable…» y «espacio de resolución»/NX-E0111 no son inferencia diferida de I07c y no se incluyen.)

## 2. §4.1.1 (inferencia monomórfica diferida)

| ID | Regla (cita) | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| S1-1 | §4.1.1 p.1: «En `var w=v`, la igualdad entre los tipos de `w` y `v` persiste tras resolver cualquiera de ellos.» | ver TD01–TD03 (§5) | IT `monomorphic_aliases_keep_the_first_concrete_origin`; I7A `deferred_var_aliases_unify_types` | cubierta | I7A solo comprueba código/subcausa; IT comprueba primario, relaciones y data. |
| S1-2 | §4.1.1 p.1: «compartir restricciones no comparte almacenamiento ni propiedad» (copias independientes, §6.1) | TD06: aceptado; gemelo vivo `AssertEq(v,[1]);AssertEq(w,[9])` sin diagnóstico | IT `bottom_equality_arrays_and_alias_copies_keep_types_and_data_independent` | cubierta | Aserta `run_scalar(..).diagnostics.is_empty()`; no aserta explícitamente `outcome=unit` ni observaciones vacías. |
| S1-3 | §4.1.1 tabla: `Never` «fondo del retículo. `join(Never,T)=T`; no añade restricciones concretas» | TD04, TD05 aceptados | IT `bottom_equality_arrays_and_alias_copies_keep_types_and_data_independent`; I7A `never_is_bottom_for_equality_and_array_inference`; QT `solver_states_and_representatives_keep_their_distinct_rules`, `right_hand_bottom_and_taint_keep_their_algebraic_states`, `union_preserves_bottom_marker_from_either_representative` | cubierta | Ver T-TD04/T-TD05 para la parte de ejecución. |
| S1-4 | §4.1.1 tabla: «Tipo por inferir … No equivale a divergencia ni a `Never`.» y p.2 «Un inicializador divergente sin anotación no restringe el tipo del binding … Una anotación sí restringe» | RD25 (anotación fija q), TD01/TD03 | RD (RD25, RD26); IT `divergent_initializer_and_array_operand_follow_their_distinct_constraints`; DMC `m00058_does_not_invent_array_shape_from_hypothetical_write` | cubierta | |
| S1-5 | §4.1.1 tabla: «Resultado contaminado … suprime solo diagnósticos derivados; no demuestra igualdad ni borra errores independientes.» | TD07 y literal de contaminación (§5) | IT `invalid_name_barrier_and_tainted_alias_do_not_cascade`; DC `independent_numeric_causes_survive_a_contaminated_binding`; MR `tainted_statement_does_not_hide_independent_sibling_errors`; QT `partial_type_table_preserves_never_and_taint_states` | cubierta | |
| S1-6 | §4.1.1 p.2: «`missing_branch_value` es una obligación estructural pendiente, no una variable libre» | `NX-E1302/missing_branch_value` (etapa ownership) sin aceptar valor inexistente | I7A `missing_branch_value_precedes_deferred_binding_type`, `nested_divergent_left_keeps_missing_branch_value` | cubierta | Los tests usan `.any(|d| code==… && subcause==…)`: fijan existencia, no cardinalidad ni primario. Corresponde a I07a. |
| S1-7 | §4.1.1 p.2: «La aceptación … no depende del orden interno del solucionador» | — | Ver N-a | — | Se evalúa en N-a (no se duplica fila). |
| S1-8 | §4.1.1 p.3: «la primera igualdad incompatible … se denuncia en su operando … “Primera” sigue el recorrido de comprobación de §4.1.2.1; no basta comparar offsets iniciales» | — | Ver K1-b | — | |
| S1-9 | §4.1.1 p.3: «Una variable usada solo hipotéticamente puede quedar interna sin materializar un valor o slot ejecutable de tipo desconocido» | aceptación y lowering ejecutable | PN `undecided_domain_inside_numeric_selector_has_executable_type`; DG `valid_pending_comparison_results_lower_like_concrete_twins`; PD `pattern_local_exports_closed_array_element`; IT `unresolved_hypothetical_index_write_remains_lowerable` | cubierta | |
| S1-10 | §4.1.1 p.4: `adr_gap_never_bottom`: «`(DV)==true` y `true==DV` son simétricos en aceptación y restricciones» | ambos aceptados | I7A `never_is_bottom_for_equality_and_array_inference` (incluye `(DV)==true`, `true==DV`, `==()`, `[DV,false]`…) | cubierta | Los `adr_gap_*` no son oráculo; aquí el esperado (aceptación) está además en TD04 del ADR. |
| S1-11 | §4.1.1 p.4: `adr_gap_deferred_var_alias`: «`var v=DV;var w=v;v=3;w=true;` se rechaza, igual que su equivalente vivo» | rechazo NX-E0102/type_mismatch | I7A `deferred_var_aliases_unify_types`; IT `monomorphic_aliases_keep_the_first_concrete_origin` (equivalente vivo = TD02) | cubierta | |
| S1-12 | §4.1.1 p.4: «`Never`, variables pendientes y contaminación nunca se serializan en `data`, `expected_type` ni `found_type`.» | — | QT `visible_conversion_keeps_internal_states_out_of_diagnostics`, `partial_type_table_preserves_never_and_taint_states`; `Diagnostic::validate` invocado en los tests de fuzz (DC `deferred_constraint_sequences_never_reach_internal_failure`) | cubierta | Los unit tests fijan el mecanismo; ninguna propiedad general sobre programas arbitrarios comprueba que `data` no contenga un tipo pendiente (RD no lo comprueba, ver 0.1.1). |

## 3. §4.1.2 nivel normativo N(a)–N(j) y nivel I

| ID | Regla (cita) | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| N-a.1 | N(a): aceptación o rechazo «independiente del orden interno del solucionador» | misma aceptación bajo permutaciones | QT `monomorphic_solution_is_invariant_under_constraint_permutations` (6 permutaciones × compatible/contradictorio), `generated_constraint_orders_have_the_same_acceptance` (10.000 casos); IT `branch_permutation_rejects_scalar_array_conflicts`, `deferred_array_conflict_uses_concrete_type_found_later_in_either_statement_order`, `deferred_array_equality_obeys_left_operand_expected_type_in_both_orders`; DT `assignment_resolution_order_keeps_causal_leaf_primary`; PN `established_array_selector_conflict_is_independent_of_assignment_order`, `nested_selector_causes_are_stable_across_resolution_order` | cubierta | Las campañas grandes (IT `generated_type_permutations_preserve_acceptance` 300.000; `statement_permutations_match_independent_constraint_oracle` 150.000; `extensive_monomorphic_constraints_match_independent_graph_oracle` 200.000) son `#[ignore]`: fuera de la puerta. |
| N-a.2 | N(a): independiente de «una forma sintáctica equivalente que conserve las restricciones» | grupos/paréntesis y bloques `{value:…}` equivalentes | IT `grouped_operands_keep_diagnostic_identity_in_deferred_and_domain_paths` (pares agrupado/plano); DM (ignorados, 50.000 y 300.000) | cubierta | Cobertura por pares dirigidos; la propiedad amplia está solo en DM `#[ignore]`. |
| N-a.3 | N(a): «Añadir o quitar cualquier D/V conserva exactamente esa aceptación o rechazo» | aceptación invariante a D/V | Ninguno dirigido. Evidencia indirecta: diferencial normativo de supervivientes (`docs/validation/i07c-mutations/normative-classification.json`: `survivor_normative_differences: 0`, 507.000 programas) | sin cobertura | No es un test; es una medición sobre el código de la época de la campaña (base `b506d7d`), no repetida sobre HEAD. |
| N-b | N(b): «La existencia, el código y la subcausa de cada C … Se emiten únicamente conflictos con evidencia propia como C.» | ver §§6–7 | RD (152 fuentes, `check` y `run`: existencia/código/subcausa de cada C); tests de `errors.len()==1` exacto: DC `assert_single_c` (usado en `nominal_results_reach_incompatible_consumers`, `nominal_result_fixes_circular_domain_before_its_failure`, `compatible_constraints_survive_indecision_and_reject_later_conflicts`), IT `mismatch_at`, PD `rejected` | cubierta | «Únicamente C con evidencia propia» (ausencia de C espurio) NO la fija RD (0.1.1), sí los tests de cardinalidad exacta. |
| N-c | N(c): «El span primario [inicio,fin) de cada C; aplazar la resolución no autoriza a trasladarlo.» | primario exacto | RD (primario de cada C); tests con `(start,end)` exacto: DC, PD, IT `mismatch_at`/`exact_type_site`, CO, MR | cubierta | |
| N-d.1 | N(d): «conservación de todos los sitios causales independientes» | varios C conservados | NS `independent_causal_sites_survive_deferred_resolution`, `complex_condition_and_selector_keep_three_independent_c_sites`; CO `independent_operands_keep_their_causal_sites`, `reduced_independent_sites_remain_causal`, `deferred_array_siblings_keep_independent_mismatches`; RD01, RD64, RD128 | cubierta | |
| N-d.2 | N(d): «Como máximo un diagnóstico por sitio» | sin duplicados | IT `assert_no_duplicate_diagnostics` (invocado por `rejected()`), `deferred_sites_emit_once_and_keep_independent_siblings`, `deferred_join_has_one_diagnostic_per_source_site`; RD (rechaza dos diagnósticos con el mismo primario) | cubierta | |
| N-d.3 | N(d): «Cada hoja incompatible con un tipo esperado concreto impuesto desde fuera de la unión es un sitio independiente» | RD08/RD14/RD34/RD59/RD61/RD64/RD128: dos C | RD (ambos C exigidos por conteo); IT `expected_context_reports_each_independent_branch_and_element`, `expected_scalar_checks_each_concrete_array_branch` | cubierta | `generated_expected_context_matches_recursive_leaf_model` (100.000) es `#[ignore]`. |
| N-d.4 | N(d): «La denuncia única por operación, o por unión sin tipo esperado externo determinado … corresponde al primer incompatible en orden fuente» | una E0102 por `+`/`-`; una E1303 por unión | CO `numeric_operation_reports_only_first_invalid_domain`, `numeric_operation_keeps_one_domain_rejection`; MR `numeric_visit_owner_limits_deferred_joins_to_one_domain_error`; IT `nested_branch_anchor_reports_the_first_incompatible_leaf_once` (literal RD74); RD15, RD16, RD88, RD91, RD100 | cubierta | |
| N-e.1 | N(e): orden de emisión de §13.9 | lista ordenada | NS `independent_causal_sites_survive_deferred_resolution` y DC `failed_nominal_withdraws_only_dependent_diagnostics` (comparan la lista ordenada); RD solo compara el orden como instantánea informativa | cubierta | No hay test dirigido de la regla de ordenación de §13.9 como tal; el orden se fija en listas de varios diagnósticos. |
| N-e.2 | N(e): determinismo (misma salida en ejecuciones repetidas) y D/V no duplicados «por sitio» | — | Duplicación: ver N-d.2. Determinismo entre debug y release: solo las campañas informativas (`docs/validation/F0-0002-I07c-estado.md`) y la propia ejecución de la puerta en ambos perfiles | sin cobertura | No hay test que repita una compilación y compare bytes; no localizado test de D/V duplicados distinto de N-d.2. |
| N-f | N(f): «Ausencia de errores internos, pánicos e `InvalidIr`» | 0 pánicos, 0 `InvalidIr`, 0 `internal_error` | DC `deferred_constraint_sequences_never_reach_internal_failure` (3 semillas × 256), PD `deferred_pattern_and_self_read_variations_do_not_panic` (aserta `!InvalidIr`), IT `structured_deferred_check_and_run_match_live_twins` (128 pares), `run_deferred_check_run_fuzz`; SD `unresolved_array_match_does_not_loop_on_option_pattern` | cubierta | Fuzz del contrato (≥400.000 release / ≥100.000 debug) es campaña externa (estado.md), no puerta. |
| N-g | N(g): corpus protegido TD01–TD07, literales de §14.10, DG01–DG08 | ver §5 | ver §5 | — | Filas T-… y G-… |
| N-h.1 | N(h): «NX-E0103/NX-E0113 sobre constructores irresueltos con obligación propia de §4.1.2.2» | un diagnóstico por constructor | CO `two_unresolved_constructors_keep_two_obligations`; MR `unresolved_constructors_close_from_their_own_obligations`, `rd127_constructor_requires_its_own_element_resolution`, `linked_array_constructors_keep_each_causal_inference_site`; DT `both_unresolved_constructors_retain_own_site`; IT `unresolved_array_alias_cycle_keeps_both_constructor_obligations`; RD101, RD119, RD122, RD127 | cubierta | |
| N-h.2 | N(h): «La omisión de un conflicto no serializable nunca permite aceptar una incompatibilidad segura: queda el C del constructor pendiente.» | rechazo con U | MR `unresolved_array_conflict_preserves_constructor_diagnostic`, `unresolved_array_element_branch_conflict_rejects_before_lowering`; IT `failed_constructor_does_not_add_a_dependent_inference_error`; RD101, RD119 | cubierta | Parte NX-E0113: MR `unresolved_result_constructor_names_missing_component`. |
| N-i.1 | N(i): «todos los spans relacionados corresponden a fuente y evidencia reales; ningún `expected_type` relacionado apunta al propio primario» | sin relación fabricada | Por instancias, con lista exacta de `related_spans`: DC `assert_single_c`, PD `rejected`, IT `mismatch_at`/`exact_type_site`, `deferred_intrinsic_domain_has_no_self_expected_origin`, `intrinsic_domain_does_not_fabricate_an_expected_type_span`, CO `intrinsic_condition_origin_stays_unrelated_after_compatible_assignment`, `intrinsic_numeric_prefix_has_no_source_expected_relation`, MR `alias_of_intrinsically_fixed_binding_keeps_no_source_origin` | cubierta | Solo por instancias. |
| N-i.2 | N(i) como propiedad general (cualquier programa) | — | Ninguno: `Diagnostic::validate` no comprueba `expected_type != primario`; RD no compara relaciones | sin cobertura | Los fuzz de la puerta no la verifican. |
| N-j.1 | N(j): D/V usan «código y subcausa definidos por este ADR … primario … sitio fuente existente, nunca … vacío» | primario válido no vacío | RD (cada diagnóstico real: código en {E0102,E0103,E1303} y `0<=a<b<=len`); `Diagnostic::validate` en DC fuzz | cubierta | RD no comprueba que subcausa corresponda a la operación del D/V. |
| N-j.2 | N(j): «Se omite obligatoriamente todo V cuya emisión exigiría serializar un tipo con variables pendientes … RD101 y RD119» | solo U, sin V | IT `failed_constructor_does_not_add_a_dependent_inference_error`, `unresolved_inner_array_suppresses_outer_constructor_consequence`; MR `unresolved_array_conflict_preserves_constructor_diagnostic` (cardinalidad exacta) | cubierta | RD101/RD119 en RD no detectaría el V extra (0.1.1). |
| I-a | I(a): presencia o ausencia de D y V | instantánea | RD imprime `SNAPSHOT` solo como informativo | sin cobertura | Nivel I por diseño: no bloquea. Se lista para que B/D confirmen que no se pretende fijar con aserción. |
| I-b | I(b): `related_spans` y `data.expected_type/found_type` de D/V y de todas las filas RD | instantánea | Ídem; donde existen aserciones exactas son tests dirigidos (ver §6) | sin cobertura | La instantánea RD no falla la puerta; los valores exactos solo están fijados en las filas con test dirigido. |
| I-c | I(c): cardinalidad entre hojas D | instantánea | RD83 (C1+D2): D no aserto | sin cobertura | Nivel I. |

## 4. §4.1.2 cláusulas 1–7 (nivel regla; ejemplos literales en §5)

Convención RD: «RDxx» significa que la fila está en `rd_table_matches_check_and_run` (N: aceptación, código, subcausa y primario de cada C; ver 0.1.1). La asignación de cada RD a una cláusula sigue la columna «Regla» de la propia tabla del ADR (el redactor del borrador la leyó; cuando la columna cita varias cláusulas se agrupa por la primera).

### Cláusula 1 — Ocurrencias y orden (N)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K1-a | «Dos lecturas de un mismo binding son sitios distintos; dos visitas internas a la misma lectura no lo son.» | RD104 (`AssertEq(r,r)`: dos diagnósticos); CO `two_pending_array_domains_keep_distinct_use_sites`, `late_binary_domain_failure_does_not_erase_later_argument_mismatch`; IT `deferred_join_has_one_diagnostic_per_source_site` | cubierta | |
| K1-b | «El orden normativo es el recorrido de comprobación … contexto heredado antes de sus hijos, hijos en orden escrito y relación del resultado … después; no se ordena un padre antes de sus hijos por tener menor offset» | IT `deferred_resolution_changes_the_first_incompatible_source_constraint`, `distinct_constraint_order_pairs_keep_code_and_primary`, `structurally_distinct_expected_context_pairs_keep_leaf_primary`; CO `comparison_rejects_once_at_first_cross_type_conflict`, `recursive_shape_planning_keeps_causal_sites_stable` | cubierta | |
| K1-c | «Las anotaciones y firmas imponen el contexto de su uso, no una prioridad global sobre restricciones de sentencias anteriores.» | CO `first_equality_origin_survives_later_alias_assignments`, `guarded_comparison_keeps_expected_origin_after_domain_resolution`; RD108, RD112 | cubierta | |
| K1-d | «los paréntesis de agrupación nunca amplían su primario» (+ contrato §3.5: «Un solo punto determina el span primario … se prueban de forma estructural») | IT `grouped_operands_keep_diagnostic_identity_in_deferred_and_domain_paths`; RD80a–d, RD81a–c (grupos en ambas posiciones) | cubierta | Ver K1-d2. |
| K1-d2 | Contrato §3.5: propiedad estructural «un solo punto determina el primario de un operando» | `scripts/i07c_span_audit.py` (lista de usos de spans crudos). No referido por ningún test ni por `scripts/validate.sh` | sin cobertura | Es un script manual (aparece en `F0-0002-I07c-implementation-notes.md`, HANDOFF.md, RECOVERY.md); no es puerta. |
| K1-e | «Una igualdad entre tipos compuestos es atómica: un conflicto no conserva unificaciones parciales» | QT `i07c_survivor3_failed_union_restores_both_constructor_roots`, `i07c_survivor3_failed_result_union_rolls_back_earlier_payload_bind`, `i07c_survivor3_failed_atomic_union_preserves_quota_error`, `i07c_survivor3_failed_concrete_union_expansion_restores_root`; SD `partial_constructor_shape_conflicts_at_the_later_alias`; RD75 | cubierta | Los `i07c_survivor*` son regresiones de mutantes (campañas ccf/S4): fijan el mecanismo interno, esperados de implementación. |
| K1-f | «Las restricciones independientes de otros hijos sí se conservan.» | CO `failed_outer_array_keeps_nested_element_cause`, `nested_union_does_not_erase_later_array_element_cause`; IT `failed_array_element_keeps_independent_sibling_error` | cubierta | |

### Cláusula 2 — Fijación y conflicto (N; I para datos)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K2-a | «Las igualdades compatibles se acumulan, incluidas las que solo relacionan incógnitas o determinan un constructor sin determinar todos sus componentes.» | DC `compatible_constraints_survive_indecision_and_reject_later_conflicts`; IT `deferred_constraints_cross_branches_loops_arrays_and_equality`; RD70, RD75 | cubierta | |
| K2-b | «La restricción que fija un componente es la primera igualdad compatible … se conserva la procedencia a través de aliases» | TD01–TD03 (T-…); CO `compatible_array_alias_keeps_first_concrete_element_origin`, `first_indexed_array_assignment_owns_later_element_origin`; MR `array_assignment_keeps_first_element_origin_after_conflict` | cubierta | |
| K2-c | «El tipo concreto de un parámetro también fija un argumento pendiente … una asignación posterior incompatible se denuncia en su RHS, conservando como origen el tipo escrito en la firma» | RD108, RD115, RD124; IT `contextual_origins_follow_first_array_element_annotation_return_and_formal`, `expected_type_origins_survive_binding_group_index_and_union_paths` | cubierta | La relación de RD108/RD124 (origen en firma) no la aserta RD; sí IT (nombre; no leído en detalle, ver §9). |
| K2-d | «Una igualdad incompatible se rechaza, sin sustituir el tipo establecido, sin desconectar las igualdades anteriores y sin convertir todo el componente en error. Se continúa con los sitios independientes posteriores.» | TD03; IT `failed_array_element_keeps_independent_sibling_error`; MR `later_unrelated_assignment_keeps_earlier_branch_diagnostic` | cubierta | |
| K2-e | «`[]` establece `Array<α>` … un escalar incompatible con ese constructor no puede fijar `α`» | RD01, RD02, RD50, RD51, RD78 | cubierta | |
| K2-f | «Un conflicto de forma o una igualdad recursiva … puede conocerse antes que los tipos completos … se completan **sus** tipos con las igualdades compatibles posteriores» | RD54, RD55, RD75; DC `deferred_array_reports_its_first_incompatible_element`; SD `recursive_component_has_no_finite_type`, `partial_constructor_shape_conflicts_at_the_later_alias`, `deep_array_equality_reports_the_incompatible_leaf` | cubierta | |
| K2-g | «Nunca se usa una asignación rechazada para cambiar el tipo de lecturas anteriores.» | RD75; CO `later_scalar_assignment_keeps_first_concrete_array_origin`, `nested_array_assignment_keeps_first_concrete_origin` | cubierta | |
| K2-h | Omisión del conflicto con tipo pendiente «solo si el componente pendiente emite NX-E0103/NX-E0113 al cierre … ni se acepta el programa» | RD101, RD119; ver N-h.2 | cubierta | |
| K2-i | «La obligación de resolver el elemento de un constructor Array se comprueba siempre al cierre … Esto incluye `[q]` … y `[DV]`» | RD127a, RD127b; MR `rd127_constructor_requires_its_own_element_resolution`; IT `all_never_array_elements_reject_without_inventing_an_element_type` | cubierta | |
| K2-j | «Misma indeterminación … un diagnóstico por constructor … dos `[]` unificados y nunca resueltos emiten dos NX-E0103» | RD122; CO `two_unresolved_constructors_keep_two_obligations`; MR `linked_array_constructors_keep_each_causal_inference_site` | cubierta | |
| K2-k | «Un consumidor contaminado por el constructor inválido o por el conflicto pendiente no añade otro diagnóstico derivado» | IT `unresolved_inner_array_suppresses_outer_constructor_consequence`, `failed_constructor_does_not_add_a_dependent_inference_error` | cubierta | |

### Cláusula 3 — Dominios y proyecciones (N; I para datos)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K3-a | «Un requisito concreto, como `Bool` de una condición o `I64` de un índice válido o de `+`, es una igualdad y puede fijar una incógnita.» | RD25, RD31, RD36; IT `subtraction_passes_intrinsic_i64_context_through_branch_values` | cubierta | |
| K3-b | «Un dominio no unitario … por sí solo no elige un tipo ni introduce componentes frescos»; predicado pendiente queda en el operando original | RD86, RD87, RD90, RD111; PN `undecided_domain_positive_controls_stay_accepted` | cubierta | |
| K3-c | Proyección condicional: contexto Bool anterior + asignación posterior `[3]` → `NX-E0102/type_mismatch` sobre `3`, esperado Bool, encontrado I64 | RD23, RD92, RD93, RD106, RD107; PN `nested_projection_activates_the_earlier_bool_context`; MR `projected_alias_keeps_earlier_element_context_when_array_arrives_later` | cubierta | Origen/relación de estas filas es instantánea (I) y no la aserta RD. |
| K3-d | Proyecciones anidadas por nivel; «un contexto escalar sobre el resultado final no inventa ninguno de los constructores intermedios» | RD46, RD47, RD107; PN `nested_selector_causes_are_stable_across_resolution_order`, `nested_projection_activates_the_earlier_bool_context` | cubierta | |
| K3-e | Base no indexable determinada después → `NX-E0102/not_indexable` sobre la base, `expected_type=domain:array`, `found_type` = tipo de la base, sin relación `expected_type` | RD11, RD12, RD84, RD113; IT `late_scalar_projection_keeps_the_selector_domain_site`, `contaminated_index_base_keeps_selector_error` | cubierta | RD no aserta data ni relaciones; las aserciones exactas de data/relación de `not_indexable` están en DC (filas b2d, §5) y CT/IT. |
| K3-f | «El requisito `I64` del selector depende de que la base sea indexable; los errores internos de la expresión del selector no» | RD84a/b, RD114, RD117, RD121; MR `invalid_nested_selector_preserves_independent_len_domain`; DMC `independent_numeric_selector_operand_survives_failed_base` | cubierta | |
| K3-g | Contexto intrínseco `I64` del selector se distribuye si la base se confirma Array (RD114) frente a asignación con contexto variable (RD96a) | RD114, RD96a, RD96b | cubierta | Solo RD (primario/código). |
| K3-h | «Un selector inválido no proporciona evidencia del tipo del acceso a consumidores exteriores» | RD71, RD114; DT `invalid_pending_selector_does_not_supply_projection_type` | cubierta | |
| K3-i | `Len` con argumento no Array: `not_indexable`, primario y `found_type` sobre el argumento (RD86, RD125) | RD86, RD125; CT `adr_gap_primitive_domain_primary_is_first_offending_argument` | cubierta | El test CT es `adr_gap_*` (solo primario/subcausa de `Len(a: 1)`). |
| K3-j | `Print`/`Inspect`/`Len` no infieren tipo; `Inspect` → `NX-E0102/resource_required` sobre el argumento, sin relación `expected_type` | RD90, RD111; MR `print_resource_uses_its_only_operand_as_primary_and_found`, `mock_read_records_requires_the_resource_argument` | cubierta | |

#### «Cierre de dominio indeciso (precisión normativa, 2026-11-01)» y ejemplos literales

| ID | Literal del ADR | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| L3-1 | `F(var q=DV;Print(q);)` | aceptado, sin concretar q | PN `undecided_domain_positive_controls_stay_accepted` (`Print(q);` sobre `var q=DV;`) | cubierta | |
| L3-2 | `F(var q=DV;let z:I64=Len(q);)` | aceptado | PN `undecided_domain_positive_controls_stay_accepted` | cubierta | |
| L3-3 | `F(var q=DV;let z:I64=(Len(q)<2);)` → M sobre `Len(q)<2`, I64/Bool | un único C | PN `undecided_domain_preserves_independent_nominal_result` (código, subcausa, token del primario, cardinalidad 1; también `check_scalar` rechazado) | cubierta | Con el literal exacto solo se aserta (código, subcausa, primario). `data` y relaciones solo se aserta en las variantes de DC `nominal_results_reach_incompatible_consumers` (`DV<Len(q)` etc., que no son el literal). |
| L3-4 | `F(var q=DV;let a=(Len(q)==true);)` → M sobre `true` | un único C | PN, ídem L3-3 | cubierta | Ídem. |
| L3-5 | `F(var q=DV;let z=((q==q)==1);)` → M sobre `1` | un único C | PN, ídem | cubierta | Ídem. |
| L3-6 | `F(var q=DV;let z=[1,(Len(q)<2)];)` → M sobre `Len(q)<2` | un único C | PN, ídem | cubierta | Ídem. |
| L3-7 | Positivos: anotación Bool; `true` comparado a `1`; `1` comparado a `true`; primer elemento `true` | aceptados | PN `undecided_domain_positive_controls_stay_accepted` (`let z:Bool=(Len(q)<2);`, `let a=(Len(q)==1);`, `let z=((q==q)==true);`, `let z=[true,(Len(q)<2)];`) | cubierta | |
| L3-8 | «Si se añade `q=3;` a los casos con `Len(q)`, falla su dominio sobre ese argumento q … No hay NX-E0103 por q sin constructor» | X sobre q | RD86a (`let a:Bool=Len(q);q=3;`) | cubierta | Solo RD (no se añade `q=3;` a los literales L3-2…L3-6 exactos). |
| L3-9 | «No se aplica esta disponibilidad nominal a una proyección cuyo tipo de elemento se desconoce, a un constructor con obligación propia de resolución incumplida, ni a una expresión `Never`» | no ancla | PN `pending_len_result_cannot_anchor_branch_during_shape_planning`, `failed_first_branch_does_not_anchor_later_branch`, `resolved_condition_failure_cannot_anchor_array_homogeneity` | cubierta | |
| L3-10 | «un fallo posterior … retira esa evidencia dependiente … no retira restricciones independientes (RD99/RD116)» | solo se retira lo dependiente | DC `failed_nominal_withdraws_only_dependent_diagnostics`; RD99, RD116 | cubierta | |
| L3-11 | Nominal en asignación que lee su destino: **válido** `fn Main(){var r=[];r=[(r==r)==true];}` → `check` checked, `run` unit, `diagnostics=[]`, `events=[]`, `observations=[]` | aceptado | PD `self_read_assignment_exports_nominal_result` (`accepted()`: `check` y `run`, `Outcome::Unit`, diagnósticos y eventos vacíos) | cubierta | PD no aserta `stdout=[]` en ese helper. |
| L3-12 | **Inválido** `fn Main(){var r=[];r=[(r==r)==true];r=[1];}` → un C: M, primario `[39,40)` (`1`), `{expected_type:Bool, found_type:I64}`, relaciones `expected_type=[21,35)`, `found_type=[39,40)`, `rule`, `cause_chain=[]`, etc. | rechazo | PD `self_read_assignment_exports_nominal_result` (`rejected()` con todos los campos, `run_entry` false y true); DC `deferred_array_reports_its_first_incompatible_element` | cubierta | DC solo aserta `any(...)` sobre la relación `expected_type`; PD aserta lista exacta. Positivo (`1`→`true`): PD línea de `r=[true]`, DC. |
| L3-13 | **Recursivo** `fn Main(){var r=[];r=[r];}` → único C NX-E0103 `[16,18)`, `types.infer` | rechazo | PD `self_read_assignment_exports_nominal_result`; DC `unresolved_constructor_keeps_its_own_site_and_accepts_a_finite_element` (con positivo `r=[1]`); SD `recursive_component_has_no_finite_type` | cubierta | |
| L3-14 | **Gemelo negativo** `fn Main(){var r=[true];r=[1];}` → M `[26,27)`, relación `expected_type=[16,22)` | rechazo | PD `self_read_assignment_exports_nominal_result` (último `rejected`) | cubierta | |
| L3-15 | **Gemelo válido** `fn Main(){var r=[true];r=[true];}` («contrasta una sola hoja con el válido») | aceptado | No localizado: ni PD, DC, SD, IT ni RD contienen `var r=[true];r=[true];` | sin cobertura | Falta el positivo pareado del gemelo negativo. |
| L3-16 | Variantes: `b1a=ir1` (`…r=[true];`), `ir1c` (`var r=[];r=[r==r];r=[true];`), `ir1d` (`var r=[];var w=[];r=[w==w];w=[true];`), «Sin asignación posterior», `ir1_t` (`var r=[true];r=[(r==r)==true];`) | todas aceptadas | PD `self_read_assignment_exports_nominal_result` (lista de 7 cuerpos; las cinco variantes están literalmente) | cubierta | |
| L3-17 | «Igualdad que decide el dominio del resultado que la alimenta» (circular b2d en 3 DV): primario `[148,149)`, `not_indexable`, `{domain:array / I64}`, `related=[found_type:[148,149)]` | un único C | DC `nominal_result_fixes_circular_domain_before_its_failure` (`assert_single_c`: campos completos, `errors.len()==1`, rechazo en `check_scalar` y `compile_scalar`) | cubierta | |
| L3-18 | Tabla de cierre: b2d + positivo `[a0,1]`; par (1) `[67,68)` + positivo; par (3) `[69,70)` + positivo; consumidor derivado `…let c:Bool=a0;…` `[67,68)` con V `[81,83)` omitido | cada negativo un único C; positivos aceptados | DC `nominal_result_fixes_circular_domain_before_its_failure` (los seis negativos y cinco positivos; incluye `let r=3;…` y el consumidor anterior) | cubierta | El «V omitido» se fija por `errors.len()==1`; los esperados coinciden con los del ADR (comprobado a mano). |
| L3-19 | «Un constructor Array independiente de ese resultado fallido cuyo elemento siga irresuelto conserva su NX-E0103 propia» | U conservada junto al C circular | No localizado un test con b2d + `[]` independiente | sin cobertura | Frase de la precisión sin literal propio. |

### Cláusula 4 — Contexto y cardinalidad (N; I para D/V)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K4-a | «Una **hoja** es cada continuación de un `if`, cada valor de un bloque `{value:...}` y cada elemento de un literal de array … contexto determinado cuando su raíz es un escalar o constructor conocido» | IT `expected_context_reaches_every_value_position_with_deferred_alias`, `expected_nested_array_reports_scalar_leaf_type`, `expected_context_reaches_else_block_value_with_and_without_deferred_alias`; PN `nested_branch_array_keeps_only_independent_union_causes` | cubierta | |
| K4-b | «Cada hoja incompatible con un tipo esperado concreto impuesto desde fuera de la unión conserva su propio C» | RD08, RD13, RD14, RD34, RD59, RD61, RD64, RD128; ver N-d.3 | cubierta | |
| K4-c | Sin contexto externo Array, «el primer elemento no divergente y con evidencia válida establece la referencia … Cada elemento incompatible posterior se denuncia por separado» | RD01, RD02, RD38–RD45, RD48–RD53; IT `heterogeneous_array_keeps_first_concrete_element_and_independent_errors`; CO `concrete_array_element_owns_homogeneity_origin` | cubierta | |
| K4-d | Unión sin contexto: referencia = primera continuación con evidencia válida; **una sola NX-E1303** sobre la primera posterior incompatible, con aplanado de `if` anidados | RD91, RD100, RD74; IT `concrete_branch_join_reports_the_later_value` (E1303 con relaciones `expected_type`/`found_type` exactas para dos fuentes), `nested_branch_anchor_reports_the_first_incompatible_leaf_once` (literal de RD74) | cubierta | |
| K4-e | «Si esa unión interna discrepa, su primera continuación válida aporta obligatoriamente el testigo parcial … dos uniones incompatibles producen dos NX-E1303» | RD65–RD67, RD73, RD100; CO `failed_inner_join_preserves_first_valid_branch_for_outer_join`, `failed_inner_join_keeps_partial_witness_for_outer_join`; MR `failed_inner_union_keeps_outer_incompatible_continuation`, `nested_later_union_uses_its_first_valid_leaf_against_outer_anchor` | cubierta | |
| K4-f | Hojas D (referencia derivada de otra hoja sin contexto externo): «La implementación de referencia también denuncia las hojas D» | RD83 (C1+D2) | sin cobertura | Presencia D es I (I(a)/I(c)); el C1 de RD83 sí está en RD. Se anota por completitud. |
| K4-g | «Disponibilidad por nodo (precisión normativa, 2026-11-01)»: la raíz se prueba al entrar en el nodo; activación tardía solo por selector/proyección | PN `outer_index_context_makes_later_flat_array_an_ineligible_twin`, `branch_with_existing_array_element_context_uses_concrete_mismatch`, `nested_projection_activates_the_earlier_bool_context`; RD96a/b; S4 `contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli` | cubierta | S4 compara contra salida de la base B1b (0.1.3), no contra el ADR; fila de deuda `I07C-S4-VS-BASE` (4 vs 3 diagnósticos). |
| K4-h | `+`/`-`: «solo se denuncia el primer operando con evidencia válida incompatible con `I64`»; el segundo pendiente fija I64 (RD126); asignación posterior propia | RD15, RD16, RD88a/b, RD94a–d, RD126; CO `numeric_operation_reports_only_first_invalid_domain`, `two_failed_numeric_operands_do_not_supply_an_assignment_type`; MR `contaminated_inner_addition_keeps_outer_right_domain`, `inner_right_numeric_conflict_does_not_consume_outer_domain` | cubierta | |
| K4-i | `<`, `==`, `AssertEq`: «igualdad antes del dominio»; conflicto de igualdad → operando derecho/segundo completo; si coinciden pero el dominio falla, primario = token del operador | RD09, RD10, RD17, RD18, RD81, RD97–RD99, RD102, RD103, RD116, RD120; IT `deferred_less_than_blames_right_operand_after_resolution`, `nested_comparison_uses_inner_operator_at_same_type_domain_failure`; MR `less_than_related_variables_use_the_operator_site_after_equality`, `repeated_deferred_less_than_uses_operator_token`, `assert_eq_array_child_compares_at_second_operand` | cubierta | `tests/f0005_b1b.rs::f5_numeric_operator_domains_match_both_adr_operand_orders` aporta lo análogo para el perfil F5 (no ADR F0-0002). |
| K4-j | Relación `found_type` de `<`/`==`: operando «determinado» en la síntesis; no la reemplaza la igualdad posterior | RD116 (`q<r`); CO `equality_domain_relation_points_to_resolved_operand`, `guarded_comparison_keeps_expected_origin_after_domain_resolution`; MR `projected_comparison_preserves_expected_operand_origin` | cubierta | |
| K4-k | Operando contaminado: se suprime la igualdad cruzada, se comprueba el dominio del otro (primario = token del operador; esperado `I64`/`domain:copyable_data`) | RD94, RD95, RD98, RD105; CO `failed_comparison_keeps_independent_numeric_right_operand`; MR `failed_comparison_right_keeps_determined_left_domain`, `deferred_comparison_keeps_valid_operand_domain_after_sibling_failure` | cubierta | |
| K4-l | Intrínsecas: primario y `found_type` sobre la expresión completa del argumento, «sin etiqueta ni grupos» (`Len(a: 1)` → solo `1`, RD125) | RD125; CT `adr_gap_primitive_domain_primary_is_first_offending_argument` (también `AssertEq(a: MockOpen(1), …)`) | cubierta | `adr_gap_*`; RD125 aserta primario. |
| K4-m | `AssertEq`: tras la igualdad, cada argumento fuera de `domain:copyable_data` es su propio diagnóstico (RD104) | RD104; MR `assert_eq_resource_reports_each_noncopyable_argument`, `resource_assert_eq_errors_suppress_result_annotation`, `assert_eq_resource_constraint_blames_second_deferred_operand` | cubierta | |
| K4-n | «si la igualdad anterior fija el tipo de un operando pendiente … conserva como origen el testigo fuente de ese otro operando … sin relación `expected_type` en el diagnóstico de dominio (RD104, RD105, RD109)» | RD104, RD105, RD109; MR `assert_eq_deferred_array_keeps_later_concrete_origin`; CO `equality_domain_relation_points_to_resolved_operand` | cubierta | |
| K4-o | `Never` conserva la excepción simétrica de §4.6 | IT `bottom_never_hides_independent_operator_or_array_mismatch`, `deferred_intrinsic_left_suppresses_independent_domain_consequence` | cubierta | |

### Cláusula 5 — Clasificación y contaminación causal (N para C/D/V, aceptación y conservación de C; I para presencia D/V)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K5-a | «Un binding cuyo inicializador falló es una declaración inválida … incluso si tiene anotación» | RD98; MR `tainted_annotated_let_invalidates_later_uses`, `tainted_call_argument_suppresses_dependent_result_mismatch` | cubierta | |
| K5-b | «El fallo de un uso de un binding con inicializador válido no contamina por sí solo ese binding ni sus demás ocurrencias»; «Una asignación fallida tampoco invalida el tipo ya establecido del destino» | MR `valid_later_call_argument_does_not_clear_earlier_taint`, `failed_assignment_condition_keeps_prior_array_constructor_obligation`; IT `tainted_array_element_retains_independent_constraint_error` | cubierta | |
| K5-c | «Una operación con operandos incompatibles no ofrece su tipo nominal de resultado para denunciar después la anotación, condición, igualdad o llamada que la contiene» (RD21/22/76/77) | RD21, RD22, RD76, RD77; IT `invalid_comparison_result_does_not_feed_an_enclosing_union`, `deferred_equality_error_suppresses_outer_annotation_error`, `pending_comparison_domain_suppresses_dependent_result_annotation`; MR `failed_assert_eq_argument_suppresses_result_annotation_only` | cubierta | La omisión del V se fija por `len==1` en los tests IT/MR, no por RD. |
| K5-d | Array heterogéneo y unión discrepante «no ofrecen un tipo único a su consumidor» (RD03/04/42/43) | RD03, RD04, RD42, RD43; IT `scalar_array_with_inconsistent_elements_reports_only_leaf`, `tainted_array_constructor_does_not_add_a_parent_shape_error` | cubierta | |
| K5-e | Testigo parcial: una unión errónea solo ofrece testigo a **otra unión**, no a igualdad/argumento/elemento/selector/anotación/`return`; como primer elemento de array no establece la referencia (RD123) | RD19, RD20, RD110, RD118, RD123; PN `failed_first_branch_does_not_anchor_later_branch`; DT `pending_right_comparison_domain_does_not_supply_outer_union_type`, `pending_branch_context_error_does_not_accuse_its_union` | cubierta | |
| K5-f | Clasificación en orden: V si «su evaluación necesita el tipo o el valor de una expresión ya denunciada o … contaminado» (o base no indexable, RD84); C por hoja con contexto externo; «Solo es **D** una hoja incompatible adicional de una misma restricción causal» | RD64/RD128/RD82/RD83 (parte C); etiquetado D/V: ninguno | cubierta | Solo la parte C (existencia y primario) está fijada; el etiquetado D/V y la presencia son I y quedan sin aserción (ver I-a, I-c). |
| K5-g | «un conflicto descubierto tarde puede suprimir una consecuencia exterior descubierta antes» (estados convergidos) | DC `nominal_result_fixes_circular_domain_before_its_failure` (fila del consumidor `c:Bool=a0` anterior), `failed_nominal_withdraws_only_dependent_diagnostics` | cubierta | |
| K5-h | «Comprobar el dominio de un operando válido … no necesita el resultado del otro operando contaminado: ese diagnóstico es independiente y se conserva» | RD94, RD95, RD98, RD105; MR `deferred_comparison_distinguishes_cross_equality_from_inner_join` | cubierta | |
| K5-i | Elemento de array contaminado no aporta; NX-E0103 de la base derivado omitido (M00474/M00475) y scrutinee contaminado omite NX-E1325 derivado (M00655) | SD `survivor_m00474_check_diagnostics`, `survivor_m00475_check_diagnostics`, `survivor_m00655_check_diagnostics` | cubierta | Esperados «de salidas de referencia» (0.1.3); `RESULTADO.md` (ccf): el ADR §4.1.2.5 «no trae un ejemplo normativo que los fije literalmente». |

### Cláusula 6 — Datos y procedencia (N: código, subcausa, primario, formato, no fabricar; I: contenido de relaciones/valores en RD y en D/V)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K6-a | «Cada ocurrencia … produce como máximo un diagnóstico de tipos»; «La propagación a hojas sustituye, no añade, la denuncia de su envoltura» | ver N-d.2; IT `deferred_if_inside_array_reports_each_site_once`, `deferred_join_has_one_diagnostic_per_source_site` | cubierta | |
| K6-b | «NX-E0102 conserva la subcausa de la comprobación original; NX-E1303 tiene `subcause=null`» | `mismatch_at`, `exact_type_site`, `concrete_branch_join_reports_the_later_value` (E1303 con subcausa `None`) | cubierta | |
| K6-c | «`data.expected_type` describe el contexto de ese sitio y `data.found_type` el tipo completo»; «Descomponer una igualdad … no autoriza a serializar el tipo del elemento con el span del array completo» | IT `array_conflicts_report_the_complete_expression_type`, `deferred_array_conflict_keeps_whole_operand_types`; MD `result_component_mismatch_reports_complete_written_types`; DC `compatible_constraints_survive_indecision_and_reject_later_conflicts` | cubierta | |
| K6-d | «Para una hoja denunciada, primario y relación `found_type` señalan esa hoja sin grupos» | IT `grouped_operands_keep_diagnostic_identity_in_deferred_and_domain_paths`; RD80, RD81 | cubierta | |
| K6-e | Origen de la relación `expected_type`: anotación/firma; asignación → «su origen concreto, siguiendo aliases hasta el inicializador o la restricción posterior que lo fijó»; comparación → operando izquierdo; homogeneidad → primer elemento; unión → operando de la primera continuación | TD01–TD03, DG07/DG08 (§5); CO `first_array_constraint_owns_later_assignment_origin`, `concrete_array_element_owns_homogeneity_origin`, `compatible_array_alias_keeps_first_concrete_element_origin`; IT `then_value_relates_the_written_expected_type`, `nested_synthesized_if_uses_the_written_value_operand_as_origin`, `union_origin_tie_uses_first_child_in_source_order`; CO `indexed_element_origin_survives_compatible_whole_assignment` | cubierta | La parte de RD es instantánea (I) y no la aserta RD; estas filas se apoyan en tests dirigidos. |
| K6-f | «Un requisito intrínseco no adquiere origen fuente por haber fijado una variable … conserva la ausencia de relación `expected_type`»; «Nunca se usa como `expected_type` relacionado el mismo operando denunciado»; «No se añade `context`» | CO `intrinsic_condition_origin_stays_unrelated_after_compatible_assignment`, `intrinsic_numeric_prefix_has_no_source_expected_relation`; MR `alias_of_intrinsically_fixed_binding_keeps_no_source_origin`; IT `deferred_intrinsic_domain_has_no_self_expected_origin` | cubierta | |
| K6-g | Resultado nominal en asignación: relación `expected_type` = array completo `[21,35)` y no el elemento `[22,34)` | PD `self_read_assignment_exports_nominal_result` (`related` exacta `(21,35)`) | cubierta | |

### Cláusula 7 — Gemelo con tipo conocido (N aceptación y causales; I detalles)

| ID | Regla (cita) | Test(s) | Estado | Observación |
|---|---|---|---|---|
| K7-a | «Adelantar un tipo concreto `T` al inicializador y retirar la asignación posterior … conserva los diagnósticos de los usos comunes» | DT `deferred_expression_twins_retain_independent_causes`, `pending_equality_result_and_numeric_rhs_match_concrete_twin`, `poisoned_assignment_still_checks_rhs`; DG `pending_operation_consumers_match_concrete_causal_twins`; CO/MR pares RD(a)/(b) (RD01/02, RD03/04, RD05/06, RD13/14, RD19/20…) | cubierta | Las campañas masivas (DM, 50.000 y 300.000 gemelos) son `#[ignore]`; `i07c_gemelos.py` es informativo (contrato §3.8). |
| K7-b | Excepciones: el adelanto cambia la disponibilidad de contexto externo (no es gemelo) | PN `incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin`, `outer_index_context_makes_later_flat_array_an_ineligible_twin`; DT `deferred_array_outcomes_have_absolute_acceptance_oracles` | cubierta | |
| K7-c | «No existe una regla de sustituir retrospectivamente cada variable pendiente por el tipo escrito en su última asignación» | RD70, RD75; MR `later_unrelated_assignment_keeps_earlier_branch_diagnostic` | cubierta | |

## 5. Literales protegidos: TD, §14.10 de fondo, contaminación, DG (N(g))

| ID | Literal del ADR | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| T-TD01 | `fn main(){var v=DV;var w=v;v=3;w=true;}` | NX-E0102/type_mismatch, `types.check`, primario `true` final, `{I64/Bool}`, `expected_type`→`3`, `found_type` | IT `monomorphic_aliases_keep_the_first_concrete_origin`: primario `[68,72)`, expected `(64,65)`, data, relaciones exactas, `validate()` | cubierta | Offsets recomputados para `fn Main` y DV de 37 bytes (0.1.5). |
| T-TD01p | Positivo: último `true`→`4` | aceptado | IT (mismo test) | cubierta | Aceptación bajo orden inverso / otro representante / debug-release: ver T-TD-ord. |
| T-TD02 | `fn main(){var v=3;var w=v;v=3;w=true;}` | primario `[32,36)`, `expected_type`=`[16,17)` | IT (mismo test): `(32,36)` y `(16,17)` | cubierta | Corrige la divergencia que declara §14.10 (`[24,25)` del código antiguo). Offsets idénticos al ADR (sin DV). |
| T-TD02p | Positivo `w=4` | aceptado | IT (mismo) | cubierta | |
| T-TD03 | `fn main(){var v=DV;var w=true;w=v;v=3;}` | primario `[65,66)`, `expected_type`=`true` de `w` | IT (mismo): `(71,72)`, `(60,64)`, data `Bool/I64` | cubierta | Offsets recomputados. |
| T-TD03p | Positivo `3`→`false` | aceptado | IT (mismo): `v=false` | cubierta | |
| T-TD-ord | TD01–TD03 positivos «también con restricciones procesadas internamente en orden inverso, con otra elección de representante y con recompilación debug/release» | aceptados | Solo propiedades genéricas (QT `monomorphic_solution_is_invariant_under_constraint_permutations`, `generated_constraint_orders_have_the_same_acceptance`, `solver_states_and_representatives_keep_their_distinct_rules`; la puerta corre debug y release). Ningún test aplica permutaciones a las fuentes literales TD01–TD03 | sin cobertura | Cobertura genérica, no de los literales. |
| T-TD-cli | «TD01–TD03 rechazados no ejecutan primitivas; `outcome.kind=rejected`, salida 1 … `rule`/`explanation_id`, cadenas `[]`, campos sintácticos nulos» | `check`/`run` CLI | IT usa `compile_scalar` (rechazo + `validate()` + `cause_chain`/`repair` vacíos); no hay test CLI para TD01–TD03 ni se aserta `rule`/`explanation_id`/`feature_id` para ellos | sin cobertura | Los campos `rule`, `feature_id` etc. sí se aserta en DC `assert_single_c` y PD `rejected` para otras filas. |
| T-TD04 | `fn main(){let x=(DV)==true;}` y `fn main(){let x=true==DV;}` | aceptados | IT `bottom_equality_arrays_and_alias_copies_keep_types_and_data_independent` (`((DV))==true`, `true==DV`); I7A `never_is_bottom_for_equality_and_array_inference` | cubierta | Doble paréntesis en IT. |
| T-TD05 | `fn main(){let x=[DV,true];}` y `[true,DV]` | aceptados | IT (mismo test) | cubierta | |
| T-TD04e | TD04/TD05: «al ejecutar retornan unidad sin observaciones» | `run`: unidad | No hay test que ejecute TD04/TD05 (`compile_scalar` sin `run_scalar`) | sin cobertura | El `run` de TD06 vivo sí se ejecuta (S1-2). |
| T-TD06 | `fn main(){var v=DV;var w=v;v=[1];w=[2];w[0]=9;}` aceptado; gemelo vivo con `assert_eq(v,[1]);assert_eq(w,[9]);` | aceptado / ejecución sin diagnósticos | IT `bottom_equality_arrays_and_alias_copies_keep_types_and_data_independent` | cubierta | Ver S1-2 (no aserta `unit`). |
| T-TD07a | `fn main(){let x=missing;let y=x;let z=true+1;}` | un NX-E0101 en barrera `names`, nada derivado | IT `invalid_name_barrier_and_tainted_alias_do_not_cascade`: `len==1`, código `NX-E0101`, `Stage::Names`, `Operation::NamesResolve` | cubierta | |
| T-TD07b | Con `missing`→`0`: NX-E0102 sobre `true`, `I64/Bool`, alcanza `types` | rechazo | IT (mismo): `mismatch_at(… (32,36), None, "Bool","I64")` | cubierta | |
| T-F1 | `fn main(){let x=DV+true;}` y `fn main(){let x=true+DV;}`: M sobre el `true` no perteneciente a DV, `{I64/Bool}`, `found_type` sobre ese operando, sin `expected_type` fuente | rechazo | IT `bottom_never_hides_independent_operator_or_array_mismatch`: `(54,58)` y `(16,20)`, `expected=None` | cubierta | |
| T-F1p | Positivo «sustituir ese `true` por `1`» (`DV+1`, `1+DV`) | aceptado | No localizado (`DV+1`/`1+DV` literales ausentes en IT, I7A, DC) | sin cobertura | |
| T-F2 | `[DV,1,true]` y `[1,DV,true]` en un binding: M sobre el último `true`, relacionado con el primer `1` | rechazo | IT (mismo test): `(57,61)` con origen `(55,56)` y `(17,18)` | cubierta | |
| T-F2p | Positivo del último `true`→`1` (`[DV,1,1]`, `[1,DV,1]`) | aceptado | Solo `[DV,1]` (I7A `never_is_bottom_for_equality_and_array_inference`), no el literal | sin cobertura | |
| T-C1 | `fn main(){let x=true+1;let y=x;let z=false+1;}`: dos NX-E0102 sobre `true` y `false`, `I64/Bool`, sin diagnóstico sobre `y` | rechazo | DC `independent_numeric_causes_survive_a_contaminated_binding` (campos completos, relaciones `found_type` exactas, `[16,20)` y `[37,42)`); IT `invalid_name_barrier_and_tainted_alias_do_not_cascade`; MR primera prueba (`fn Main`) | cubierta | |
| T-C1p | Positivos: ambos booleanos por `0`; corregir solo uno deja solo el otro error | aceptado / un error | DC (corrige uno → 1 error; `1+1` ambos → `is_ok`); IT (`0+1` ambos → aceptado) | cubierta | DC usa `1+1` en vez de `0`: equivalente, no literal. |
| G-DG07 | `fn main(){var a:Array<i64>=[0];a[0]=true;}`: M sobre `true` `[36,40)`, `expected_type`=`[16,26)` (anotación), `found_type`=`[36,40)` | rechazo | IT `indexed_assignment_uses_annotation_or_inferred_initializer_origin` (`mismatch_at` con `(36,40)`, `(16,26)`, `Bool`, `I64`, relaciones exactas) | cubierta | Con la grafía `Array<I64>`. |
| G-DG07p | Positivo `true`→`1` | aceptado | IT (mismo) | cubierta | |
| G-DG08 | `fn main(){var a=[0];a[0]=true;}`: primario `[25,29)`, `expected_type`=`[16,19)` (inicializador `[0]`), sin anotación inventada | rechazo | IT (mismo) | cubierta | |
| G-DG08p | Positivo `true`→`1` | aceptado | IT (mismo) | cubierta | |
| G-DG04 | `…let a=[0];drop(a);a[0]=5;`: un NX-E0202/definitely_consumed `[28,29)`, `data={}` (propiedad: I07b) | rechazo | IT `unavailable_index_root_precedes_derived_immutable_write` (primario, relaciones, `cause_chain`, `validate`); positivo `var a=[0];a[0]=5;` | cubierta | N(g) protege DG01–DG08; DG04–DG06 son de I07b. Positivo literal del ADR es otro (`let`→`var` y quitar `drop`): el test usa `var a=[0];a[0]=5;`, equivalente. |
| G-DG05 | `…drop(a);a=[1];`: NX-E0105 sobre `[28,29)` | rechazo | IT `complete_replacement_after_drop_still_checks_immutability` | cubierta | Sin leer a fondo (ver §9). |
| G-DG06 | `…drop(a);print(a);b=1;`: NX-E0202 `[42,43)` y NX-E0105 `[45,46)` con cadena y relaciones | rechazo | IT `independent_consumption_and_immutable_write_both_survive` (aserta ambos, orden, relaciones y cadena) | cubierta | |
| G-DG01-03 | DG01–DG03 (fallos de índice runtime R0102, `value.index`) + variantes con primer índice `2` | `simulator_fault` | `src/ir/i10b_tests.rs` (tabla con `a[0][2]` y `a[2][0]`) | cubierta | Propiedad de I10b; solo localizado por `grep`, no leído. |

## 6. Tabla RD (§14.10: 127 identificadores, 152 fuentes)

| ID | Regla | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| RD-N | «Las 127 filas (152 fuentes, RD79 retirado)»: aceptación, código, subcausa y primario de cada C (columnas normativas); `check` y `run`; salida 1 y sin ejecución | los 152 pares `check`/`run` | RD `rd_table_matches_check_and_run` (el script transcribe la tabla del ADR y compara el hash del fragmento: un cambio del ADR invalida el fixture; el estado.md registra «152/152 check/run pairs normativos» en debug y release) | cubierta | Ver 0.1.1: no detecta C espurios ni D/V. La salida 1, `outcome.kind` y ausencia de ejecución se comprueban solo vía código de salida (no `outcome.kind` ni `stdout`/`events`). Requiere `python` en PATH. |
| RD-I | Columnas I: relaciones y `expected/found` de `data` de todos los diagnósticos RD, presencia y cardinalidad D/V | instantánea | El mismo script las compara y las imprime como `SNAPSHOT` informativo, sin fallar | sin cobertura | Por diseño (nivel I). Donde hay aserción exacta es en los tests dirigidos de §§4–5. |
| RD-ejec | RD87 (a)/(b)/(c)/(d): «ejecución unidad; (c) imprime `[3]`» | `run` | RD solo aserta exit 0 y ausencia de diagnósticos; no aserta lo impreso | sin cobertura | Salida impresa de RD87c (`[3]`) no comprobada. |

## 7. Constructor de patrón sobre raíz abierta (F0-0003 §3.2 / §8.2, tarea S2 de I07c; ADR distinto)

| ID | Regla / literal (F0-0003) | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| P-1 | `fn Main(){let l0=DV;match l0{Some(x)=>{},_=>{}}}` y con `None=>{}`, y con `Ok(x)` + `_` | `check` checked, `run` unit, sin salida | PD `open_pattern_shape_accepts_constructor_orders_and_shared_payloads` (`run_scalar` → `Outcome::Unit`, diagnósticos, stdout y eventos vacíos) | cubierta | |
| P-2 | `Some(Some(x))` y `Some(Ok(x))` válidos | aceptados | PD (mismo) | cubierta | |
| P-3 | `Some(x)=>{}` sin `None`: NX-E1304/non_exhaustive `[57,62)`, `missing_pattern=None`; positivo `,None=>{}` | rechazo | PD `open_pattern_reports_family_at_pattern_and_coverage_at_match` (campos completos); positivo en P-1 | cubierta | |
| P-4 | `Some(x),Ok(y),_`: NX-E1325/constructor_shape `[78,83)`, `{constructor:prelude:Ok, scrutinee_constructor:Option}` | rechazo | PD (mismo); SD `nested_constructor_shape_conflicts_at_the_second_arm`, `partial_constructor_shape_uses_canonical_message` | cubierta | |
| P-5 | `Some(Some(x)),Some(Ok(y)),_`: `[89,94)`; positivo eliminando `Some(Ok(y))=>{},` | rechazo | PD y SD (`accepts(negative.replace(...))`) | cubierta | |
| P-6 | `…let b:Bool=l0;` → NX-E0102/constructor_shape `[95,97)`, `origin [66,73)`, `expected_type [90,94)`, `found_constructor=Option`; con `Result<I64,I64>`; positivo `Option<I64>` | rechazo | DC `pattern_shape_conflicts_belong_to_later_equality` (incluye `check` y `run`, tres casos y el positivo); SD `partial_constructor_shape_conflicts_at_the_later_alias` | cubierta | |
| P-7 | `None` desnudo sin otra evidencia es binding; brazos inalcanzables: `None=>{},_=>{}` | NX-E1304/unreachable_arm `[75,76)`, `context [66,70)` | PD `open_pattern_reports_family_at_pattern_and_coverage_at_match` | cubierta | |
| P-8 | «Una forma concreta previa incompatible conserva NX-E1325/constructor_type y su `scrutinee_type` completo» | rechazo | No localizado en tests de I07c (`constructor_type` aparece en `j03_match.rs`, `j04_match_semantics.rs`, `j16_integrated.rs`, no leídos) | sin cobertura | Propiedad probablemente de F0-0003 (`j03`); no verificado. |
| P-9 | `Len(l0)` sobre forma parcial (fila `I07C-LEN-L0`): solo N(f) | sin `internal_error`/`InvalidIr` | PD `deferred_pattern_and_self_read_variations_do_not_panic`; NS `adr_gap_self_selector_without_concrete_base_keeps_current_acceptance` | cubierta | Solo N(f); el diagnóstico no está fijado por el ADR. |

## 8. Cuotas relacionadas con inferencia (§11.1, §11.8 «A. `type_solver_steps`», contrato §2)

Fuera de alcance de I07c, asignado a I10a (contrato §2): spans, subcausa y testigos de NX-E0003 bajo perfiles
reducidos más allá de «una sola NX-E0003 por ámbito con `observed = limit + 1` y el resto como subconjunto exacto».

| ID | Regla (cita) | Esperado | Test(s) | Estado | Observación |
|---|---|---|---|---|---|
| Q-1 | §11.1: `type_solver_steps` «1.000.000 globales»; unidad = «Restricciones procesadas y pares lógicos de nodos comparados»; §13.12: etapa `types`, operación `AnalysisLimit` | `NX-E0003`, `limit=1000000`, `observed=1000001`, `budget_scope=compilation`, `incomplete_analysis=types` | QT `solver_charges_the_final_granted_unit_and_reports_one_rejected_unit` | cubierta | Nivel de unidad (contador sembrado en 999.999), no por fuente completa. |
| Q-2 | Último paso concedido se acepta | aceptado con 1.000.000 | QT `default_solver_limit_accepts_its_last_granted_step`, `measured_solver_test_stack_minimum_still_accepts_last_granted_step` | cubierta | La fuente es de dos arrays anidados 100×, no la del ADR. |
| Q-3 | §11.8.A: fuente literal (`let a0=0;` + 64 envoltorios + `var y=a64;` + 15.400 `y=a64;`; 93.307 bytes; `15400*65=1001000>1000000`) en el perfil normal | NX-E0003 durante `types`, antes de propiedad, efectos e IR | Ninguno (`15400`, `a64`, `y=a64` no aparecen en `tests/`, `src/` ni `scripts/`) | sin cobertura | La cuota por fuente completa del perfil normal no se ejerce con el literal; se ejerce el cargo por unidad (Q-1/Q-2). |
| Q-4 | Perfil reducido `f0-0002-test-types-32-v2`: 64 bindings `let xK:i64=0;`, límite 32, observed 33, positivo `fn main(){}` | NX-E0003 con `limit=32`, `observed=33`, span válido por propiedad | SD `solver_budget_rejects_first_step_over_thirty_two` (`ReducedProfile::Types32`, `run_entry` false/true, propiedad del span); `tests/analysis_profiles_and_quotas.rs::reduced_analysis_source_frontiers_are_transcribed_from_the_profile_table` (frontera 15/16/17) | cubierta | Es perfil reducido (I10a es el dueño de su exactitud); se anota por completitud. |
| Q-5 | Contrato §2: «una sola NX-E0003 por ámbito con `observed = limit + 1` y el resto de diagnósticos como subconjunto exacto de la ejecución sin cuota» | un solo NX-E0003; resto = prefijo/subconjunto | QT `reduced_type_solver_profile_keeps_exact_converged_subset`, `quota_does_not_hide_converged_later_capability_error`, `nested_array_value_keeps_one_global_quota_error`, `tainted_array_stops_after_the_first_denied_charge`; `src/check/i07c_quota_campaign.rs::reduced_quota_property` y QT `generated_multifunction_quota_has_one_denied_charge_and_exact_prefix` (ambos `#[ignore]`) | cubierta | La propiedad masiva está `#[ignore]` (estado.md: `quota_reduced_release` 4.800 programas, «informativo»). |
| Q-6 | Unidades de cargo (§11.1) por recorrido: pares de nodos, hojas, restricciones diferidas | contador exacto en el límite reducido | QT `contextual_array_charges_each_leaf_once_at_the_reduced_boundary`, `cached_arrays_charge_every_logical_pair_at_the_reduced_limit`, `nested_unification_charges_each_array_pair_at_the_reduced_boundary`, `deferred_if_join_charges_the_reduced_boundary`, `deferred_array_selector_keeps_exact_quota_charge_site` | cubierta | Los tres `adr_gap_*` de cuota (`adr_gap_union_charge_at_the_exact_reduced_boundary`, `adr_gap_shape_walk_charge_per_constructor_link`, `adr_gap_unresolved_assert_eq_charges_each_deferred_constraint`) fijan comportamiento actual, no oráculo; el ADR no enuncia estas cifras (andamiaje según AGENTS.md). |
| Q-7 | Escala lineal de pasos/asignaciones del solucionador diferido (criterio del estado.md: razón ≤ 4,6) | razón de escala | IT `deferred_unification_scale_is_linear_in_steps_and_allocations`, `nested_assert_eq_unification_scale_tracks_depth_and_source_bytes` (ambos `#[ignore]`); `tests/i07c_mutant_projection_scale.rs::resolved_projection_does_not_repartition_unrelated_pending_constraints` (`#[ignore]`) | cubierta | Campaña explícita, fuera de la puerta. El criterio 4,6 es del estado.md, no del ADR. |
| Q-8 | Profundidad inferida de arrays (`array_type_depth` 128, 127/128/129) bajo inferencia diferida | aceptar 128, rechazar 129 | QT `inferred_array_depth_accepts_127_and_128_and_rejects_129`, `inferred_array_path_keeps_array_depth_priority_over_value_depth`; IT `inferred_array_depth_has_a_deterministic_analysis_limit`, `late_array_constructor_obeys_the_inferred_root_depth_limit`; MD `inferred_array_wrappers_preserve_both_result_component_depths`; SD `expression_and_signature_depth_boundaries_keep_parse_and_types_separate` | cubierta | |

## 9. Resumen, trabajo propuesto e incertidumbres

### (a) Recuentos por estado

Cada fila de las tablas de las secciones 1–8 cuenta una vez (se excluyen las filas que solo remiten a otra,
marcadas con «—» en la columna de estado: S1-7, S1-8, N-g). Recuento hecho con un script de lectura sobre las tablas de
las secciones 1–8 (cada fila con exactamente una de las tres etiquetas).

| Estado | Filas |
|---|---:|
| cubierta | 143 |
| cubierta solo por mutación | 0 |
| sin cobertura | 19 |
| Total contado | 162 |

Sobre «cubierta solo por mutación = 0»: no es una afirmación de que ninguna regla dependa de mutantes. Los
mutantes de las campañas no están etiquetados por regla del ADR (`population.json`/`results.jsonl` identifican la
línea de producción y el test que mata), y la asignación mutante → regla no se hizo (ver incertidumbres). Lo que sí
hay son regresiones nacidas de mutantes (tests `i07c_survivor*` de QT, `survivor_m*` de SD, `i07c_mutation_regressions.rs`,
DMC, MD), que aquí cuentan como tests dirigidos (K1-e, K5-i, etc.), con la salvedad 0.1.3.

### (b) Trabajo propuesto (filas «sin cobertura» y la fila de mutación)

1. **L3-15**: añadir el gemelo válido `fn Main(){var r=[true];r=[true];}` (aceptado, `check` y `run`).
2. **T-F1p / T-F2p**: positivos de fondo literales (`DV+1`, `1+DV`, `[DV,1,1]`, `[1,DV,1]`) y **T-TD04e**: ejecutar TD04/TD05
   con `run` y comprobar unidad y observaciones vacías.
3. **T-TD-ord / T-TD-cli**: permutar internamente las fuentes TD01–TD03 (orden inverso, otro representante) y un test CLI
   (`check`/`run`: salida 1, `outcome.kind=rejected`, `rule`, `feature_id`, sin ejecución) para TD01–TD03 y DG07/DG08.
4. **N-i.2 / N-j.2 / N-b (extras) / RD-N**: reforzar `scripts/i07c_rd_conformidad.py` para que falle ante un diagnóstico C
   espurio (código/primario no esperado) y ante cualquier `data` con tipo pendiente, y para que compare `outcome.kind`,
   `stdout`/`events` en `run`; o añadir una propiedad general («ningún `expected_type` relacionado coincide con el primario»)
   en el fuzz de DC/IT. Es la laguna estructural que más condiciona el resto.
5. **Q-3**: test con la fuente literal de §11.8.A en el perfil normal (93.307 bytes) comprobando NX-E0003 en `types`;
   hoy solo se ejerce por unidades.
6. **L3-19**: ejemplo con b2d más un constructor `[]` independiente que conserve su NX-E0103.
7. **K1-d2**: promover `scripts/i07c_span_audit.py` a test de la puerta (o equivalente estructural) o documentar que el
   criterio 5 del contrato se cumple solo por revisión manual.
8. **N-a.3 y N-e.2**: decidir si basta el diferencial normativo (informativo) o se exige un test dirigido de «añadir/quitar D/V
   no cambia aceptación» y de determinismo entre ejecuciones; hoy no hay ninguno dirigido.
9. **P-8**: localizar o crear el test de `constructor_type` con `scrutinee_type` completo en el contexto de raíz abierta.
10. **I-a / I-b / I-c, RD-I, K4-f, RD-ejec**: nivel I por diseño; B/D deben confirmar que no se exige aserción (el ADR permite
    actualizar la instantánea con autorización del coordinador).
11. **Mutación (pendiente, no fila)**: los 95 `sin_programa_tras_caza` de ccf y los 47 de S4 (`Pending.md`: `I07C-MUT-SIN-PROGRAMA`,
    `I07C-S4-MUT-SIN-PROGRAMA`), más los 309 supervivientes de la población `b506d7d` (305 informativos y 4 equivalencias normativas,
    estado.md) no están asignados a ninguna regla de esta matriz; B/D deberían mapear al menos los de `src/check/types.rs` a las
    filas N(b)/N(c)/N(d) antes de afirmar cobertura por mutación. La campaña extendida está pausada (`I07C-MUT-EXT`).

### (c) Incertidumbres (lo que no pude comprobar)

1. No se ejecutó nada: no sé si todos los tests citados pasan hoy, ni si compilan sin avisos. Solo comprobé que existen
   (listados por `grep` y, para los marcados, leídos).
2. **Leídos en detalle** (aserciones comprobadas): DC (completo salvo 130–280), PD (1–345), PN (1–135), SD (244–395 y 495–540),
   `assert_single_c`, `mismatch_at`, `exact_type_site`, `assert_no_duplicate_diagnostics`, los tests IT de TD/DG/fondo (3255–3430, 3038–3090,
   3184–3250, 790–822, 3513–3545), I7A `never_is_bottom…`/`deferred_var_aliases…`, QT cuota (18118–18228) y permutaciones (18719–18800),
   CO `pending_len_domain_keeps_independent_result_consumers`. **Citados por nombre** sin leer su cuerpo (el nombre sugiere la regla, pero no
   verifiqué sus aserciones): la mayoría de CO, MR, DT, DG, DMC, MD, NS, IT fuera de las zonas citadas, los `i07c_survivor*` de QT, `src/ir/i10b_tests.rs`,
   `tests/f0007.rs` y `tests/corpus.rs` más allá de lo transcrito. B/D deben leerlos antes de dar una fila por «cubierta».
3. La asignación de cada RDxx a una cláusula la hice con la columna «Regla» de la tabla (leída entera para RD01–RD128 vía fixture y
   ADR); algunas filas citan varias cláusulas y la agrupación es del redactor.
4. No comprobé que los offsets de las filas TD/DG de IT coincidan con la recomputación manual del ADR (0.1.5), salvo TD02 (idéntico) y
   DG07/DG08/TD04–TD07 que no usan DV (verificables por inspección).
5. No localicé tests de §4.1 que no sean de inferencia (`Result` copiable, NX-E0111 no invocable): excluidos de la matriz.
6. No leí `scripts/i07c_*` de campañas (gemelos, tipada, dominios, normativa) ni sus resultados más allá de `estado.md` y
   `normative-classification.json`; su valor como evidencia de cierre (contrato §3.3, §3.4, §3.6) no se valora aquí.
7. No consta en el repositorio un mapa mutante → regla del ADR; «cubierta solo por mutación = 0» es consecuencia de no haber hecho ese
   mapa, no de que no existan filas apoyadas en mutación.
8. `docs/validation/i07c-mutations/` (población `b506d7d`) y las poblaciones ccf/S4 son anteriores a los cambios posteriores de `types.rs` (p. ej. la fila `I07C-MUT-EXT` de `Pending.md` los llama «pre-S4 y pre-F5-pila»); el estado.md indica que cada cambio de producción obliga a regenerarlas. No verifiqué que sigan vigentes sobre `0650fee`.
9. El conteo de la sección (a) se calculó con un script que lee las tablas 1–8; si se edita la matriz, recontarlo.
