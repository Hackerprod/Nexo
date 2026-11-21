# I07c-S4-r1 — implementación C, paso 4.3

**Estado de C:** cambio de producción y pruebas terminados; la puerta completa y el veredicto de D corresponden al coordinador y al validador. El blob actual de `src/check/types.rs` tiene SHA-256 `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`.

Base de trabajo `7439f21` (S4.1 `8cd5afe` más documentos). Sin commit ni push; Git se usó solo para lectura. Propuesta validada `I07c-S4-r1.md` §3; B dio PASS a la familia y D dio PASS a la propuesta.

## Código y regresiones

- `Checker::check_against_context_one` (`src/check/types.rs`): el hecho hijo de `Binary`/`Index`/`Call`/`Array` se fusiona una sola vez justo después de `expr_inner` o `PrecomputedType::Whole`; `result?` y el resto de la comparación usan el hecho padre. Se quitaron las fusiones tardías en error, proyección pendiente y salida normal. No se cambió ningún otro símbolo de producción.
- Instrumentación exclusiva de `#[cfg(test)]` en `InferenceGraph::begin_visit`, `child_fact`, `fact_depth`, `Checker::resume_non_match` y `check_against_context_one`: siembra controlada de owner/taint, traza de aristas y observación de síntesis, comparación y cierre de marcos. No existe en el binario de producto.
- `quota_tests::boundary_context_owner_closes_constructor_before_comparison` (prueba interna de `types.rs`): observa el hijo con owner vacío y dos hechos activos durante la síntesis, luego el padre con owner heredado y un solo hecho activo antes de `unify_with_expected_origin`. Repite con owner `Some((26,41))` y `None`; terminó verde.
- `tests/i07c_s4_context_regressions.rs`, registrado en `tests/suite.rs`: oráculo CLI completo de `be8c47c`, con el nombre temporal del archivo normalizado a `case.n`, para el negativo y positivo pareados de §1 y las cuatro fuentes de D (`Index`/selector, `StatementSite`, RHS de asignación y valor de rama `If`). Se comprueban `check` y `run`, exit 1, stdout JSON completo y stderr vacío. Terminó verde. `tests/f0005_b1b.rs` no se editó.

## Diferencial dirigido

Semilla 4071; generadores y runner versionados en `docs/validation/i07c-s4/andamiaje/` con SHA-256 y receta de reproducción en su README. Corpus principal: 3.072 fuentes, SHA-256 `854578a62fe37895cd8a98c7062ac2bd5fcd910e88c6035745ce92b720171758`; ampliación `Index`: 384 fuentes, SHA-256 `6c4071d6249a1c804d766bcd6bd0efdf18d60a95178d64ce21afc2c809d07433`. Identidad de ejecutables congelada antes de ejecutar:

| Perfil | `be8c47c` | `3100a29` | Corregido de este árbol |
|---|---|---|---|
| debug | `cc85a4b99eca7ed734e52ea60b226196dc5e6392af5b315a636a6dc4c8e1bd76` | `6edf41c63b45fbd00bd7d0acf43901f5539d463a60fa5cfd10f2c68ae0930169` | `ab55cc3cbfb77e3c09e7e459be01df420f5650da0e8d482f27f91f708acedc2c` |
| release | `80b6d03bf1b0824b2aaca0dbce0f5e1078bc391ec931c81cebdc50e6241212c6` | `e8aabcf7b94d3948cdad0d15ae74e8d847f8df68899b52c41bab58f9a9d6b112` | `970c9c8f02e99d4bbb2ba665482c24fa687e6fec5b1bf93e3ba9c1b65674d5fc` |

La pasada final, congelada antes de ejecutarse sobre los binarios de la tabla, comparó por separado ambas bases con el corregido, con `--profile f0-0005-b-v1` explícito, `check` y `run`, debug y release: **3.456 fuentes, 41.472 invocaciones, cero diferencias** en exit/stdout JSON/stderr, cero timeouts y cero pánicos/`internal_error`/`InvalidIr` detectados. El corpus principal duró 481,8 s y la ampliación `Index` 64,3 s. Los scripts versionados y portables tienen SHA-256 `e41a4da818f32f0067f4f0fe12f26c54001df5f71ad0814c5fb3973e58e65f25`, `16f7124db624d97f8fe1a44079951cfc12fb0e3fe1a9386d13eea671c8f6b792` y `5d2f1a5fb1e6d709f8e0704ec91dbb45800d0d5a0c39a2f6aa88f75f857f906d`; los corpus conservaron sus hashes congelados. Esta sonda no sustituye la puerta completa del coordinador.

## Mutaciones inversas

Blob corregido de `src/check/types.rs`: SHA-256 `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`. Las 20 inversiones de §5.3 se aplicaron una por vez en una copia desechable. Cada fila verificó prehash, SHA-256 del mutante y posthash restaurado al mismo blob. `cargo nextest run --profile default --retries 0` inició el universo sin filtro de 2.128 tests debug y paró en el primer fallo (exit 100); por la regla de parada no se ejecutó release de los mutantes muertos. Se ejecutaron los 20 tests dirigidos con el blob final: cada uno falló en una aserción de `types.rs` (exit 101, 0 pasados/1 fallado), sin error de compilación, sintaxis ni pánico incidental. El corregido pasa esas pruebas en la puerta rápida.

| Inversión | SHA-256 mutado | Test interno dirigido que falla | Primer test del universo `default` que falla |
|---|---|---|---|
| `array_owner` | `0b3aab8adeccc674a28c9a962bddf2e226c53ce62378a546e7f2f80e7b75beb5` | `boundary_array_and_index_restore_owner_before_consumers` | `boundary_array_and_index_restore_owner_before_consumers` |
| `asserteq_owner` | `827aeab8342bd417c7cf1e64d810c3c8341789643bad7090269402ebd8b1324f` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `assignment_taint` | `7730973cd4d3049167c06ef286a1182a096431bc0d4d3687fb70dcf17a44139e` | `boundary_taint_edges_clear_before_local_values` | `boundary_taint_edges_clear_before_local_values` |
| `binary_deferred` | `4b848e21ea11c11093ed8fdfb53cf46b8f8c3650911c34d6500bc99a8475d594` | `boundary_binary_deferred_rhs_keeps_owner_until_result` | `boundary_binary_deferred_rhs_keeps_owner_until_result` |
| `binary_independent_right` | `df213d1b1e2e491f56d6aba1417f15d64d1859843e2d2fbf341b3a310fb843fa` | `boundary_binary_context_closes_after_prepared_comparison` | `boundary_binary_context_closes_after_prepared_comparison` |
| `binary_left` | `5df9fd4c9842557c516a881e81536151b10a313bfbf4af0890483a43cbb3a1cc` | `boundary_binary_left_quota_keeps_operator_until_context_close` | `boundary_binary_left_quota_keeps_operator_until_context_close` |
| `binary_left_error` | `11ee5e79e0a95197ece7e45cca9765649b03066cb362bc39e0c182f323febe07` | `boundary_binary_context_closes_after_prepared_comparison` | `boundary_binary_context_closes_after_prepared_comparison` |
| `binary_right` | `64afb2411867d1e4fba76490362267d9d4b310fb7853fe6491b19455c5cb05ee` | `boundary_binary_context_closes_after_prepared_comparison` | `boundary_binary_context_closes_after_prepared_comparison` |
| `call_owner` | `25c3b3b8209747f5a90279769164e0043ef78e5f5161daf3c633ef11809a771f` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `calltarget_owner` | `e7fce3a5d0d761ce2f4815ee2213f847da7f797153b442fbd03b184779f7a4a3` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `closure_statement_taint` | `b3805e21640d4a0117dab6c9c7a34f651fffa48990a48b04b84671eca17c8827` | `boundary_taint_edges_clear_before_local_values` | `boundary_taint_edges_clear_before_local_values` |
| `closurecall_owner` | `53572f230fc7af69252d1a137e286940a34c0f71759d9438f84fe4a635aacef8` | `boundary_closure_call_restores_parent_fact` | `boundary_closure_call_restores_parent_fact` |
| `if_branch_taint` | `2a66bdc1f5982c804bf78ae16dc94bb47c1caa065b33305a44b41dc7321af4dd` | `boundary_taint_edges_clear_before_local_values` | `boundary_taint_edges_clear_before_local_values` |
| `index_owner` | `a18e60f021a28572dcd9a6a40505b9c9b74338fe0028960f3daccf95b6d18bb5` | `boundary_array_and_index_restore_owner_before_consumers` | `boundary_array_and_index_restore_owner_before_consumers` |
| `intrinsic_owner` | `6b5b41a98a774178df4e29cdfe5026801108b2bf96c9c677493ef94cb2fc1c9c` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `match_branch_taint` | `83983aef0e5a61d09700b78a91af7190e88a25e9a03ff942452857aaf90dfaf3` | `boundary_taint_edges_clear_before_local_values` | `boundary_taint_edges_clear_before_local_values` |
| `statement_taint` | `1113fb401ca9f053c9e4f5606360f8cc13986fae51b714134e3dbe5185d40d30` | `boundary_taint_edges_clear_before_local_values` | `check::tests::divergent_left_keeps_independent_numeric_operand_sites` |
| `tryadd_owner` | `ea3b7dfdc125eefad1789d984e68fef31de84648f82ee41ac9da1e2b8b8b644f` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `union_owner` | `1c963fd2d330d2cb714eef4a0e223c0e6909bcc2174ba3d0c6772247bf35f532` | `boundary_frame_owner_survives_completed_calls` | `boundary_frame_owner_survives_completed_calls` |
| `context_owner` | `975aa901b31ca77b5ee974f9492bb696a898a3aa16d0ca5d878104dfc1f69e46` | `boundary_context_owner_closes_constructor_before_comparison` | `boundary_array_and_index_restore_owner_before_consumers` |

La inversión `statement_taint` puede ser detectada primero por `check::tests::divergent_left_keeps_independent_numeric_operand_sites`; su test dirigido `boundary_taint_edges_clear_before_local_values` también falló en la aserción de la arista. `context_owner` falló en `boundary_context_owner_closes_constructor_before_comparison` al observar `after_synthesis` con owner `None` y dos hechos, frente al padre esperado con owner heredado y un hecho. `tests/i07c_s4_context_regressions.rs` contiene los oráculos CLI conocidos de `Index`, `StatementSite`, RHS de asignación y rama `If`, además del negativo/positivo pareado; se ejecutaron en `check` y `run` con el perfil explícito y JSON completo.

## Puerta de iteración

`sh scripts/validate-fast.sh` terminó exit 0 tras formato, Clippy `-D warnings`, comprobación de selección y 2.106/2.106 tests debug más 2.108/2.108 release del perfil `fast`. La puerta rápida es solo evidencia de iteración; la puerta completa queda a cargo del coordinador. `git diff --check` no halló errores.

## Diferencial protegido y RD

El diferencial estricto de 369 fuentes protegidas (corpus, fixtures y las 152 RD), SHA-256 ordenado `e639eee74fa50842c04f5d009a91e5c85b7335afbd1940359def62de0ffe517a`, dio **cero diferencias** de exit, stdout JSON y stderr frente a cada base (`3100a29` y `be8c47c`), por separado, en `check`/`run` y debug/release con `--profile f0-0005-b-v1`. El verificador RD reprodujo **152/152** pares `check`/`run` en debug y release; diferencias de instantánea: 0.

## Pila A/B al perfil r4

`rustc 1.99.1` MSVC, `cargo test --lib --no-run --profile test` sin `/MAP`, ejecutables de test distintos y el mismo arnés `scripts/i07c_stack_probe/probe.py` (SHA-256 `37940070a7fa1cb0b646e14cb98c84a14d9ffbf6327b9e344fd21c4d13237f43`). Con DI: `RUSTFLAGS` y `CARGO_PROFILE_TEST_DEBUG` sin definir; sin DI: `RUSTFLAGS=-C debuginfo=0`, `CARGO_PROFILE_TEST_DEBUG=0`. Por cada árbol, perfil, modo TYPES/CHECKER y caso se midió el primer tamaño candidato estable (5/5), con el tamaño anterior fallando 5/5 por exit 3221225725. Las ocho fuentes además pasaron a 204 y 256 KiB en ambos modos.

| Caso | Base y corregido con DI | Base y corregido sin DI |
|---|---:|---:|
| `some84`, `some126` | 193 KiB | 193 KiB |
| `array126` | 129 KiB | 129 KiB |
| `array_bool126`, `none_array126` | 193 KiB | 193 KiB |
| `option_match126`, `result_match126`, `some_pattern126` | 129 KiB | 65 KiB |

Los mínimos del corregido son **idénticos** a `be8c47c` en las 32 combinaciones caso/perfil/modo, por tanto no empeoran el umbral. SHA-256 de ejecutables de test: base DI `4326fde50871a9c04af84acbb8113c79c988f8745421faa30e307a894616049b`, corregido DI `6f6398ddb160fd79a34b689b77d39a7256749b173afd6bf8b43a2520011b85c3`; base sin DI `ef5c76b8467a71915501f0519f184bc03631c77a1eee30ff2e66668575785f2f`, corregido sin DI `96db7670dcb6c2a21fa92ca70756e008e1fcaa0a9173768da143bf6055c6bcda`.

## Ticks, objetivos y símbolos retirados

Los tests `ir::i10b_tests::empty_function_charges_entry_then_final_unit`, `indexed_assignment_delta_and_result_error_include_semantic_charges`, `finite_loop_matches_the_published_source_ledger` y `published_work_table_totals_and_deltas_are_preserved` pasaron en debug y release de la puerta rápida. Sus aserciones conservan **2/6/1** ticks (función vacía, delta de asignación indexada y unidad final) y **56/25/15** (bucle finito, propagación `Result` exitosa y con `Err`). `scripts/objetivo.py` sin `--actualizar` midió **15/29** programas objetivo que ejecutan; S4-r1 corrige diagnósticos y no añade programa objetivo.

`git diff --check` terminó sin errores. El grep final en `src/check/types.rs` dio **cero** apariciones de `VisitCursor`, `solver_snapshot`, `visit.tainted`, `visit.numeric_domain_owner` y `parent_visit`. `tests/f0005_b1b.rs` permanece sin edición. La copia de mutaciones quedó restaurada al SHA-256 del blob corregido. Git se usó solo para lectura: no hubo commit, push, checkout, reset ni stash. La puerta completa de `validate.sh` y `gate.sh` corresponde al coordinador; este informe no declara PASS del corte.
