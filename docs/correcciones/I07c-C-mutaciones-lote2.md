# I07c C: mutaciones supervivientes, lote 2

Población congelada: `docs/validation/i07c-mutations-ccf/population.json`, SHA-256 `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4` (739 entradas). Código de producción S3: `11a70ec`. Este lote contiene 42 IDs de `survivors-batch2.json`. El árbol de entrega modifica únicamente pruebas `#[cfg(test)]` de `src/check/types.rs`; ningún cuerpo de producción cambia.

## Alcance de las rutas

Se instrumentaron 14 entradas de función con `eprintln!` en `C:/nexo-work/surv2/probe`, una copia desechable. La batería fuente `i07c_solver_depth` llamó a `closure_finish` (6), `closure_body_advance` (12), `capture_expression_dependencies` (8066), `seed_planned_shape` (1116), `selection_type` (668), `pattern_shape_mismatch` (48) y `result_shape_conflict` (4). `i07c_pending_nominal_domain` llamó a `pending_operation_type` (201), `pending_nominal_available` (119), `resolve_pending_projections` (1778), `require_array` (319) y `resolved_join_side` (518). Ambos binarios terminaron con 0 fallos. Estos conteos establecen cuándo se alcanza cada ruta desde programas Nexo; las nuevas pruebas internas aíslan la transición que el mutante cambia.

Grupos de regresión: resultado de cierre; estado de hijos presembrados; pistas de forma; asociación de obligaciones con su sitio; disponibilidad de dominio; propagación de dependencias de hijos de llamada, `if` y `match`; identidad de constructores y unión; profundidad de selección; conflictos de patrón y `Result`; reconstrucción de `Option` en joins. Las pruebas de profundidad y de camino de padres fijan además invariantes internos del solver.

## Resultado por ID

El SHA-256 de cada blob mutante corresponde a sustituir literalmente una entrada congelada en el `src/check/types.rs` final. El verificador confirmó **38/38 detectados** en las reejecuciones dirigidas; cuatro IDs se clasifican abajo por invariante. Las reejecuciones se hicieron mientras se añadían pruebas, por lo que los hashes de los blobs ensayados antes de la última prueba también incluyen menos líneas `#[cfg(test)]`; el literal de producción sustituido fue el mismo. Después de cada reejecución el script comparó por SHA-256 el archivo restaurado con su propio original.

| ID | Resultado | SHA-256 blob mutante |
|---|---|---|
| M00088 | matado por `i07c_survivor2_closure_restores_enclosing_result_site` | `40c47523660aa91753fd302b731183145b14b5b943998830aa0690d0f8bed9b5` |
| M00089 | matado por `i07c_survivor2_concrete_closure_result_does_not_bind_pending_return` | `9802fbe4db2c2c28e5753e872c4153d98273645c7aed67754f2dc3d074d5d4ae` |
| M00091 | matado por `closure_inferred_returns_keep_type_mismatch_diagnostic` | `3cddb3ce63135e915409ad18316eb47a7671b6690626c666aa329ab74006a19c` |
| M00098 | equivalente: si `first == start`, ambos tipos proceden de la misma primera asignación y `prior != source_type` es falso | `18bf980466012fe1f3f167be6b16536f9ba367f4d304baf62ef580a43a400295` |
| M00108 | matado por `i07c_survivor2_seeded_array_child_starts_clean_and_keeps_existing_leaf` | `b1915d6a26014da371a0dc8f2d5de0d0b42bccbaa8323a7b1a85b5f17ca2a573` |
| M00109 | matado por `i07c_survivor2_seeded_array_child_starts_clean_and_keeps_existing_leaf` | `4cc5e0539bccd08abe44d6d544c82cfd9b350331cd1f2c73c061187f3545be09` |
| M00114 | matado por `i07c_survivor2_seeded_array_child_starts_clean_and_keeps_existing_leaf` | `ec7bdd66e5720744ad106c150ce1a2a9cc76455ba67e6c24ffe1eb3987866716` |
| M00123 | matado por `i07c_survivor2_nested_shape_hints_reach_the_leaf` | `42306051dcffb2536784bfd63608f31bff88c3ed805708195ba4e2bc60fc97d6` |
| M00125 | matado por `i07c_survivor2_nested_shape_hints_reach_the_leaf` | `d670205fb7602fb79ddfa79593f0941979de8dc05fbf46737d5ee0f62ea78843` |
| M00130 | matado por `i07c_survivor2_pending_selector_keeps_independent_taint` | `85016c2f6593b7aacb60dd18abb09785a30ceb41f56a1388059c9fa973c3813f` |
| M00142 | matado por `i07c_survivor2_pending_operation_types_match_their_sites` | `be7573a69047712733697781855bb838f593ce18063b3c13982f83ec22132604` |
| M00143 | matado por `i07c_survivor2_pending_operation_types_match_their_sites` | `85cf5fce41f4d9858fe38ec37850aa422b738d6826b600f8c4a00c45b7ef3142` |
| M00145 | matado por `i07c_survivor2_pending_nominal_requires_live_domain` | `8e00454b3a0a44c9c9787bfd5540fa5bfc2932115ebe28fa015480244637d06e` |
| M00146 | matado por `i07c_survivor2_pending_nominal_requires_live_domain` | `ad7669eb91a16d1b1f20cfdfce68328c43116605ad8eb7bfe43dfc58d0a392d8` |
| M00149 | matado por `i07c_survivor2_pending_nominal_requires_live_domain` | `264bd6301e112390e2eaa7f002739483e7f4892462a2a306faa0090a4eb1e071` |
| M00151 | matado por `i07c_survivor2_pending_nominal_requires_live_domain` | `d52a1a76608b9c69728d734fb01b381f8a0f159008b6bd03d54e28affdb7c7d2` |
| M00152 | matado por `i07c_survivor2_pending_nominal_requires_live_domain` | `b2699ce13f3c6d3d987c20bbf83315ac63ae416727417b20c607292442483782` |
| M00156 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `9b2961eb0bd54afa3219ad491c5dc1a93553b4db9cd32577f208cbd234b3c207` |
| M00157 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `de31b000adda84659a7d283fb12d93af4bcad579f1c384502f8599e22f267fdd` |
| M00158 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `1985270d76c3386ed8873503628352124cf6cbf0f10660035c300af1e74242b4` |
| M00159 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `0633b46d0b376bb9a1de93228306a7319f59e472c6445a2827aaedc0bb1d5530` |
| M00160 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `3da0d3bb1dc56a65d881148dcfa841271919e5a9d850f34960d4297df3fd8e9a` |
| M00161 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `36025fa6afd736f526c67c0e2e0af35f5778acc24a69692baa0940f05dd04689` |
| M00162 | matado por `i07c_survivor2_expression_dependencies_cover_branch_children` | `745074a53648757b9ec4b109980e5ed1a12003cacfcebc0c49ab7c28d8e03f73` |
| M00169 | matado por `i07c_survivor2_solver_preserves_constructor_identity` | `685205fe4352b0c7880dca0f33aab39cc086da33cb1fde8ec48a794036e9e207` |
| M00174 | equivalente: con inicio de span igual solo cambia el origen conservado; sus consumidores consultan presencia, no ese span | `00cbefbff64797b13259ddc646fd48b984883a63a5d7caf58f492d32f408a14a` |
| M00178 | matado por `i07c_survivor2_root_compresses_the_entire_parent_chain` | `df5538c0d74638c5bca86283e90b61b9e8540973636047f685c0cded2b716524` |
| M00180 | matado por `i07c_survivor2_solver_preserves_constructor_identity` | `de8ab3d5d3861f53c4c9495383d93580866b9e431d56eb2af577c755e173eed9` |
| M00181 | matado por `i07c_survivor2_solver_preserves_constructor_identity` | `f1d32731a972f57ba3ffa48fd05fa16152254b37f111a0aa3c58293241c90eb3` |
| M00201 | matado por `i07c_survivor2_selection_depth_keeps_the_current_type` | `932838128071273e45772f53672c00f4760567cceabf1581e7fa5ab6d10aeaa1` |
| M00202 | matado por `i07c_survivor2_selection_depth_keeps_the_current_type` | `12acf27c04909029d206fe18c9efe8bdc634d8da9e698a0500297d8aa3aedcc3` |
| M00226 | sin programa: la profundidad de tipo fuente superior a 128 se rechaza antes de este recorrido | `eebea0083c33eb5fbefcd58d913e99c95f78c4553502169ad3a246875d40762f` |
| M00227 | matado por `i07c_survivor2_shape_conflicts_continue_past_irrelevant_children` | `f3504fabe947676c42bb1f354dab9c00aefd133c41a5db09b3156b0ee09deca8` |
| M00228 | matado por `i07c_survivor2_shape_conflicts_continue_past_irrelevant_children` | `51d0bf5f55df5ca6d6cc1a90f7ff27b071615c2e9c5f392df0d1091baa0474ac` |
| M00231 | matado por `i07c_survivor2_pattern_shape_chooses_first_conflicting_child` | `049c1bddda4c766bd958e0e0d6f17cda1aefeaf8a33c9434cdfa4ae1ce58f777` |
| M00234 | matado por `i07c_survivor2_shape_conflicts_continue_past_irrelevant_children` | `d7b2791bc285cf23f693e4fc0ed60ad508e5f63c0f23b11f22b7be82671e4b6e` |
| M00236 | matado por `i07c_survivor2_result_shape_conflict_keeps_payload_identity` | `987fde0318ae0a67de3bd1e20d4706a230a6003ec5ca6c3e95cd93ba4f7e776e` |
| M00237 | matado por `i07c_survivor2_shape_conflicts_continue_past_irrelevant_children` | `682471cb004fcd08e64cc5e102fb2145b4dcf75e097f4b58a3c4d97936511b6e` |
| M00243 | equivalente: las parejas de variantes canónicas de `Option` y `Result` tienen las mismas posiciones `Some`/`None` | `58310f6cddecc7487587c2ec0bda7ee12dcb2730f09807352e22502d3872a6ec` |
| M00251 | matado por `i07c_survivor2_resolved_join_rebuilds_option_payload` | `54bba01a8e7391de639cfaa907c7b50bec06b45e7da44c465ade91985b8b6d11` |
| M00252 | matado por `i07c_survivor2_resolved_join_rebuilds_option_payload` | `00d2b4598769594e78d148589fc9f6e926ba9f4c5bb9f2e8f9ef5cb1460ffe02` |
| M00254 | matado por `i07c_survivor2_resolved_join_rebuilds_option_payload` | `0cc16445803b4c85967034800026bb8ad854830ef19c57bf417ee3c6dcc76d1b` |

### Mutantes sin diferencia de programa

- `M00098`: `first_assigned_type` elige la asignación de inicio mínimo de la misma raíz. Si `first == start` para el sitio examinado, `prior` y `source_type.0` proceden de esa misma asignación; la desigualdad de tipos que acompaña a `<` es falsa. Si los tipos son distintos, sus inicios son distintos y `<` y `<=` coinciden.
- `M00174`: el desempate entre spans con el mismo `start` solo cambia cuál `Span` queda en `copyable_origin`. `bind` y `union` consultan si existe ese origen; el diagnóstico usa el operando actual, no el span almacenado, tal como explica el comentario del propio método.
- `M00226`: la rama `depth > 128` en `pattern_shape_mismatch` queda fuera del lenguaje fuente admitido: los tipos de valor y de array más profundos se rechazan en los límites de profundidad antes de que esa ruta produzca un diagnóstico de forma. No se fabrica un grafo de 129 niveles para aprobar una regresión que ningún `.n` puede activar.
- `M00243`: `UnionDescriptor::option` coloca `None` y `Some(payload)` en posiciones fijas; `UnionDescriptor::result` coloca dos payloads. Tras comprobar `union.kind == wanted.kind`, el `zip` no puede encontrar un par `Some`/`None`. La rama mutada es inalcanzable con descriptores canónicos.

## Verificación

En `C:/nexo-work/surv2/root`, el verificador se ejecuta contra `root/disposable/tree`, con `CARGO_TARGET_DIR=C:/nexo-work/surv2/root/target-surv2`, `TEMP=TMP=C:/nexo-work/tmp-surv2`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=1` desde el segundo grupo, `PATH` con `$HOME/.cargo/bin` primero, `--locked --offline`, `--mutation-profile release` y un solo proceso pesado propio a la vez. El directorio de Cargo está bajo la raíz de la copia porque `verify_integration` lo exige. La copia del script lee la población congelada y comprueba su SHA-256: su enumeración con el árbol posterior al lote 1 produce 741 candidatos, dos más que la población fijada. También se ampliaron únicamente en la copia los plazos internos de compilación de base y mutante de 180 a 600 s después de dos resultados inconclusos por plazo; cada invocación externa está limitada por `timeout 900`.

Comando de reejecución, variando la lista `--ids` indicada a continuación:

```text
I07C_MUT_DIR=docs/validation/i07c-mutations-ccf timeout 900 python scripts/i07c_mutaciones.py --base ccf6df1 --unit-prefix check::types::quota_tests::i07c_survivor2 --ids <ids> --reuse-tree C:/nexo-work/surv2/root/disposable --target-dir C:/nexo-work/surv2/root/target-surv2 --results-copy C:/nexo-work/surv2/root/results-batch2.jsonl --mutation-profile release
```

Listas verificadas con `--unit-prefix` (salida final de cada ID: `detectada`):

```text
M00142
M00146,M00149,M00151
M00088,M00108,M00109
M00114,M00123
M00125,M00152
M00156,M00157,M00158,M00159,M00160
M00161,M00162,M00169,M00178,M00180
M00181,M00201,M00227,M00228
M00202,M00231,M00234,M00236,M00237
M00251,M00252,M00254,M00143,M00145
M00089,M00130
```

`M00145` agotó inicialmente el plazo interno de compilación de 180 s; la repetición con 600 s dio `detectada`. `M00202` sobrevivió a un test que solo dejaba un frame en la pila; al añadir un array pendiente de reconstrucción, la repetición dio `detectada`. El caso fuente de cierre se verificó aparte:

```text
timeout 900 python scripts/i07c_mutaciones.py --base ccf6df1 --integration-test i07c_solver_depth --ids M00091 --reuse-tree C:/nexo-work/surv2/root/disposable --target-dir C:/nexo-work/surv2/root/target-surv2 --results-copy C:/nexo-work/surv2/root/results-batch2.jsonl --mutation-profile release
Base i07c_solver_depth: 1.25 s
M00091: detectada (closure_inferred_returns_keep_type_mismatch_diagnostic)
```

Pruebas nuevas sobre el código correcto, en la copia aislada:

```text
timeout 900 cargo test --locked --offline --lib i07c_survivor2 -- --nocapture
test result: ok. 15 passed; 0 failed

timeout 900 cargo test --locked --offline --release --lib i07c_survivor2 -- --nocapture
test result: ok. 15 passed; 0 failed

timeout 900 cargo test --locked --offline --test i07c_solver_depth closure_inferred_returns_keep_type_mismatch_diagnostic -- --exact --nocapture
test result: ok. 1 passed; 0 failed

timeout 900 cargo test --locked --offline --release --test i07c_solver_depth closure_inferred_returns_keep_type_mismatch_diagnostic -- --exact --nocapture
test result: ok. 1 passed; 0 failed
```

El blob de `git show HEAD:src/check/types.rs | sha256sum` es `539a6a570a97891956e49e599e605ffb060e69030081e8b03304c991c0807bd4`. Tras la última sincronización, árbol de entrega y ambas copias de verificación tienen SHA-256 `98c3af5c7c512548772bbf00f4a3cafb37be3ffc033d57d5b18ba2ed4172f75e`; el diff respecto a HEAD comienza dentro de `quota_tests`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

La primera repetición release tras copiar un fuente restaurado reutilizó el binario del mutante `M00151`, porque `Copy-Item` conservó una fecha anterior. Falló una prueba de dominio pese a que los bytes del fuente eran correctos. Se actualizó solo la fecha de `types.rs` en la copia y la recompilación release base pasó. Esta ejecución fallida no se atribuye al código de producción.

### Puerta mecánica en clon limpio

En `C:/nexo-work/surv2/gate`, clon limpio de `d1c1bd0` con solo los tres archivos de esta entrega copiados, `sh scripts/validate.sh` terminó con **código 0**:

```text
PATH=/c/nexo-work/surv2/bin:$HOME/.cargo/bin:$PATH CARGO_TARGET_DIR=C:/nexo-work/surv2/gate/target TEMP=C:/nexo-work/tmp-surv2 TMP=C:/nexo-work/tmp-surv2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=1 RUST_TEST_THREADS=4 sh scripts/validate.sh
exit 0
```

`cargo fmt --all -- --check` y `cargo clippy --locked --offline --all-targets -- -D warnings` pasaron. La biblioteca debug terminó con `1153 passed; 0 failed; 53 ignored` y la biblioteca release con `1155 passed; 0 failed; 54 ignored`; todas las integraciones y el binario pasaron en ambos perfiles. `i07c_solver_depth` dio `13 passed; 0 failed` en debug y release; `i07c_rd_conformidad`, `1 passed; 0 failed` en ambos. La documentación tuvo `0 failed`.

La primera invocación envolvió el script completo en `timeout 900` y fue interrumpida durante `runtime_profiles_and_resource_limits` al agotar el plazo global; no sirve como veredicto de prueba. La invocación final usó una envoltura `cargo` **fuera del repositorio** que separó las llamadas `cargo test` del script en biblioteca, binario, cada integración y documentación, cada una bajo `timeout 900`. Así se mantuvo el límite por ejecución sin cortar la puerta completa. El registro está en `C:/nexo-work/surv2/gate-validate-bounded.log`; el `CARGO_TARGET_DIR` del clon ocupó 2,51 GB. No hubo cambios de producción entre la verificación de mutantes y esta puerta.
