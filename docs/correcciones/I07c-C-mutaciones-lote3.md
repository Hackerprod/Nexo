# I07c C: regresiones de mutación, lote 3

Población congelada: `docs/validation/i07c-mutations-ccf/population.json`, SHA-256 `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4`. Los 80 IDs proceden de `D:/nexo-work/mut-s3/survivors-batch3.json`. Producción S3: `11a70ec`. Solo se añadieron pruebas `#[cfg(test)]` a `src/check/types.rs`; ninguna línea de producción cambió.

## Rutas y criterio

En una copia desechable se instrumentó `solve` antes de diseñar las pruebas. `i07c_solver_depth` alcanzó `Bind`, `RequireUnion` y `Unify`; `i07c_pending_nominal_domain` alcanzó `Unify`. Ambos terminaron sin fallos. Los casos de fuente `fn Main(){var q=(if true then {return;} else {return;});let n=Len(q);q=[1];Print(n);}` y su variante `Len(q[0]);q=[[1]]` se aceptan, pero no distinguen M00463 ni M00464. Por ello no se incorporaron como regresiones. Las pruebas internas añadidas aíslan las transiciones del solver que discriminan.

Se agruparon las rutas por función: recorrido de forma y conflictos; `solve` (`Bind`, `RequireUnion`, `Union`, `Unify` y cierres atómicos); resolución de joins y obligaciones nominales; visitante de expresiones y propagación de contexto. Para los últimos dos grupos no se encontró una regresión discriminante en los casos ensayados. «Sin programa encontrado» documenta ese límite de búsqueda, no afirma equivalencia ni PASS de esos IDs.

## Resultado por ID

`49` mutantes quedaron detectados por pruebas de lote 3, salvo M00241 que ya lo detecta una prueba del lote 2. Los otros `31` no se usaron para fabricar pruebas que pasen sin discriminación: 6 tienen un invariante de equivalencia local y 25 quedan sin programa discriminante identificado. El SHA-256 de la última columna corresponde al blob completo de `src/check/types.rs` tras aplicar el literal y reemplazo congelados al archivo final, con la conversión de fin de línea que usa `Path.write_text` en Windows. El SHA-256 del archivo correcto restaurado es `bab5d26000161f5375815b9b3206fdcae6ad2ee71b02051dc597f4cca96b2494`.

| ID | Resultado y motivo | SHA-256 blob mutante |
|---|---|---|
| M00232 | matado por `i07c_survivor3_shape_walk_checks_later_result_payload` | `1fa9f6dd32060912d92f0c1836f810191814fbf10f8a1e8298685b8a169cf61c` |
| M00238 | matado por `i07c_survivor3_shape_walk_checks_later_result_payload` | `2dfb6a78a325990c9ab50b04f12f70d45d9fb32e9174f635321f6f9a0fb8cc17` |
| M00241 | matado por `i07c_survivor2_result_shape_conflict_keeps_payload_identity` | `931c979f6b87164f41ff9dc49f4cbd089296c87a0f050ad3f654b27f60db3b9a` |
| M00258 | sin programa encontrado: Bind(Never) deja el resultado previo; no se halló una continuación que consuma un valor distinto de Unit | `b1b555795b12a346a89c889060057d7f56bc16ebc7134b65bba45ea153669115` |
| M00259 | matado por `i07c_survivor3_never_payload_keeps_later_result_bind` | `5637769bad9a41111ab290d9cbcbe9f3fc95a6451f2f56ac60d73a518e9d6a37` |
| M00265 | sin programa encontrado: salida anticipada de Bind ante conflicto concreto; no se halló un frame externo que cambie la restauración | `f250d8f6e89c8b8528637fac3bf8cdef55285175232ebd3c38f9316cf83976e6` |
| M00267 | matado por `i07c_survivor3_shape_requirements_reject_incompatible_roots` | `714d01a2ba0eef19d463fba4755b94a2845c1a5f78b8f79009b6238bfd563e21` |
| M00268 | matado por `i07c_survivor3_failed_result_bind_restores_first_payload` | `d6d9be8a8d85a1a74fd04fad32d684bb0f615bde89f5fafa9c1a63d080e1539f` |
| M00272 | sin programa encontrado: autoselección de array en Bind; no se halló continuación observable tras el error | `16f32c48cafa35848fb70e5f0a4e99c2f644bd56f8fc47ec4b60e98898ecce50` |
| M00276 | sin programa encontrado: cuota en Bind; no se halló continuación observable tras el primer error | `0583ded51199bf892fd32d5a18a87335da1a5fc854c96b3370d6a72c4f00e479` |
| M00278 | matado por `i07c_survivor3_noncopyable_second_payload_restores_first_bind` | `631dc799261cef3558da4dbeba08c17373a562e2fb149618929e3b0a74f18864` |
| M00282 | sin programa encontrado: Bind difiere un conflicto de forma; no se halló continuación que consuma el resultado previo | `610bd85f55bff378010909917bd499b2847110b10c94bf1b1f62d13b4a092f77` |
| M00286 | sin programa encontrado: Bind escalar de tipo igual; no se halló continuación que consuma el resultado previo | `5f1dc9f023e22caf89578edf018cb762adf239b91c1fcaecb57f50eb5f03631c` |
| M00287 | sin programa encontrado: Bind escalar nuevo; no se halló continuación que consuma el resultado previo | `4400e488b082be54632f4244518e4af30caa76f80535ddbdd3703412119da2e4` |
| M00288 | matado por `i07c_survivor3_require_union_quota_propagates_through_bind` | `367637b92d4aa869b15889dc299168730663de7fcb036ec2fd60fac535a96625` |
| M00289 | sin programa encontrado: cuota al entrar en RequireUnion; no se halló continuación observable tras el primer error | `9efcaf7ae658d86704bcc4188dabbc1912a27c3d511e998655508fca237a078b` |
| M00292 | matado por `i07c_survivor3_shape_requirements_reject_incompatible_roots` | `1e3e8e750d8a92b9b7d7c14136b43e042945d0fbb7a16cf95c39c6df4166e406` |
| M00294 | matado por `i07c_survivor3_shape_requirements_reject_incompatible_roots` | `b8a69577af4dbc4b1c586a14142f5bddd28cdd523230fde09c6ebd8c829968c2` |
| M00295 | sin programa encontrado: forma incompatible en RequireUnion; no se halló continuación observable tras el error | `07ca7c3384f0734c72757ae90e76d9ff8fef7f82edbb08b83f020d93a865c090` |
| M00296 | matado por `i07c_survivor3_union_child_allocation_propagates_quota_errors` | `9e4aab66828c05a9dcc5308beb1f494f6d8a2de266537d5476f62a0d15c25af0` |
| M00297 | sin programa encontrado: cuota al crear el primer hijo de RequireUnion; no se halló continuación observable | `3348da9cedad7b95252ba0384f31f0133c04cb9846d3400dd645460a97eb3037` |
| M00299 | matado por `i07c_survivor3_union_child_allocation_propagates_quota_errors` | `95a25bbb110a23ca6d10f1132b6bfa58344b40147c9b7b51d7764fd1e651c2db` |
| M00300 | sin programa encontrado: cuota al crear el segundo hijo de RequireUnion; no se halló continuación observable | `40599d4dfdb09af80bc4736f40838a9b61514c95ae2ee352b232021cd4b78d48` |
| M00302 | matado por `i07c_survivor3_require_union_moves_concrete_payloads_to_children` | `1e95c4d44cff1017ea37599e26e9ca7de40e8ec5541f69496536a231fc36972a` |
| M00304 | matado por `i07c_survivor3_failed_result_union_rolls_back_earlier_payload_bind` | `b270986544ce74bb9355ef5a132224085839a85a31a99f9bcb1062cf0735071c` |
| M00305 | matado por `i07c_survivor3_failed_result_union_rolls_back_earlier_payload_bind` | `73afb1dfcd7447d5179ab7e92ce881b71c1eb349bed33fbb354e1d1cb2390824` |
| M00308 | matado por `i07c_survivor3_late_result_payload_quota_rolls_back_first_bind` | `76f91ea1907178665e77fbfa97cba3c831e2c594c182962abe076632f0c334da` |
| M00317 | matado por `i07c_survivor3_incompatible_deferred_shapes_defer_without_recursion` | `17542c7f4ba1ef057abdc3366a9b84b9bf74331a73f3c4a6a7df4a49360834eb` |
| M00326 | matado por `i07c_survivor3_array_join_wraps_pending_operand_shapes` | `6f6fa1a8c65b6c9c58a9bd8c64fcd5c29976da37f9f88aa23036e10b03d09872` |
| M00337 | equivalente: la cuota agotada conserva el primer error; repetir charge_solver en el bucle no cambia contador ni diagnóstico | `dacb6b582ae5d90fa48cb8db819618ff2423bb7c74e8c04c7fc0707ddf36329e` |
| M00338 | matado por `i07c_survivor3_cached_array_child_rebuilds_outer_shape` | `cdb5784f542443dda81e8ed9922b921e6aaf8203a22768bb9123db0203c346c9` |
| M00342 | matado por `i07c_survivor3_failed_result_payload_restores_earlier_deferred_bind` | `7adb5e09c40cd44b6a67af88238a7d66e57aa5ce4f23ffbda15d98a34e32087d` |
| M00343 | matado por `i07c_survivor3_unify_concrete_union_kinds_and_payloads` | `4c34100ac49f94beb8bed1ee25981963af73d2eebef43d960068f5853a8d82f5` |
| M00344 | matado por `i07c_survivor3_failed_result_payload_restores_earlier_deferred_bind` | `94b8f7d8d274b930c911ca3e6d7f71f41c38f12f34a91d05a61b4fe0ee8785a3` |
| M00345 | matado por `i07c_survivor3_failed_result_payload_restores_earlier_deferred_bind` | `bdb92c747b5c190147547d3f5fe048e2ff3855b8f3c691edffda0f31583820d7` |
| M00347 | equivalente: UnionInner solo se apila después de Union exitoso, por lo que la guarda de error no se alcanza | `dbb4414e7f1d7069a78488f0ebf78dab089fe8bf4f326c629ea42c4380034b88` |
| M00351 | matado por `i07c_survivor3_union_rejects_variant_against_array_in_both_orders` | `1f0b230fa04e32ab205c1d653e4ec250e9fb64e8a255b2791002bd5abca28b9f` |
| M00361 | matado por `i07c_survivor3_union_prefers_larger_component_before_id` | `25169a66943ac4e7aa1bfd631d3b162ad37c83b7a4138b06958d92be1fa679a1` |
| M00362 | equivalente: tras el descarte previo de l == r, l < r y l <= r coinciden | `c29336cf7210a2062a7d0d6d48243c4e64ac74b629550581605ee31e70a9f44e` |
| M00371 | matado por `i07c_survivor3_union_transfers_single_sided_shape_and_origin` | `73fbc964d3d0359ca648931e8348cac0fbe7882a04ab08781370ae00694222a9` |
| M00372 | matado por `i07c_survivor3_union_transfers_single_sided_shape_and_origin` | `9f9c45a1f7892ec6bd44473f36ef640d23c2a6b16d4460e27a29b205206a51fe` |
| M00381 | matado por `i07c_survivor3_failed_union_restores_both_constructor_roots` | `ed9fc08545ac6a8d0d44804536bc13247273e6bc90f88d39e0fdb4d51fd6a256` |
| M00382 | matado por `i07c_survivor3_failed_atomic_union_preserves_quota_error` | `c9fcec341330f70afb13c6ec3dbc5ad12da290e918209715c2fa7b9418690fc6` |
| M00383 | matado por `i07c_survivor3_failed_concrete_union_expansion_restores_root` | `b90285fb355501f3d734d9687f44141e8e0156e91beb758ac3b83795b90412b5` |
| M00385 | matado por `i07c_survivor3_rebinding_known_union_keeps_first_shape_origin` | `5cbacb7f543c8305bf02e25985814365cb693a6377d2552164a2ce23f9c3c57f` |
| M00386 | matado por `i07c_survivor3_completed_option_uses_binding_origin` | `820a775592f17f5b028749bdb1de85749d2b34414beb0cd06614f1dc59ec0edf` |
| M00387 | sin programa encontrado: BindUnionFinish conserva el resultado previo; no se halló consumidor que lo distinga de Unit | `863783258ed695510badccf2272e3e73d1edeb3122820bbdf0c97485225c5aa7` |
| M00388 | matado por `i07c_survivor3_failed_concrete_union_expansion_restores_root` | `9012c1ba000cbb814207391f592e62505f323ee51880a8b7203095fdb65e89c6` |
| M00389 | matado por `i07c_survivor3_require_union_moves_concrete_payloads_to_children` | `bfbbe729c6c3cfaf8261173a2d5a21c5364e6b82559572f7fe02883836ae9c2b` |
| M00391 | matado por `i07c_survivor3_failed_result_union_rolls_back_earlier_payload_bind` | `8ef59775b8cc7d9d3e5aea1a39ee8848541c1de3f013dd97729be8587c135fbd` |
| M00408 | equivalente: el desempate de copyable_origin solo cambia el span almacenado; los consumidores consultan su presencia | `fc610d0002e23b29d445538da918ef36663506034891d377497c52dc6c4fc11a` |
| M00409 | matado por `i07c_survivor3_equal_start_array_origins_choose_left_diagnostic_span` | `2f266de7f6669fa3f632208ad571b0ad19f20cb91805c816980ad8a90123efea` |
| M00410 | matado por `i07c_survivor3_equal_start_concrete_origins_choose_left_diagnostic_span` | `fd399b079960a0e35023c87f14205a46e2664b765617e1efd59f1b93293170ab` |
| M00411 | equivalente: con concrete=None, Bind sobrescribe concrete_intrinsic antes de cualquier lectura diagnóstica | `9f4a634bdf0df44068159198851484581abf418c7dbde978cfe6cc4349f408d3` |
| M00413 | sin programa encontrado: UnionMerge conserva el resultado previo; no se halló consumidor que lo distinga de Unit | `ab383199c4950e59d6128b80a54f33382bde5b34fe4ad990ba4454303d58ef0f` |
| M00416 | matado por `i07c_survivor3_nested_partial_result_deferral_keeps_array_shape` | `29ff8940ec9099f864331257b664929b9a2d100d379d3802ac7acd1a5d26fe14` |
| M00418 | matado por `i07c_survivor3_nested_partial_result_deferral_keeps_array_shape` | `1d78426d4f67ed8e379f7b21da89fcad427691db04ffe66e7b6665926fcff945` |
| M00427 | matado por `i07c_survivor3_nested_partial_result_deferral_keeps_array_shape` | `f0672d6c056b8248baba17af5be7f8227d5ca53e12fd9576f6918de4a04ba17b` |
| M00428 | matado por `i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands` | `fa64d1cebe89daa1f799d6d6bb60f6055b5085ff0d26d4dc1a0d65eb59b995d0` |
| M00429 | matado por `i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands` | `614cd4c4c264e79b63152b97cf640465075d0cbfa03d883316b64fd788b1dbcd` |
| M00430 | matado por `i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands` | `bc6d86e55512ab3d5167892c8c3338097e57b7d71dd42f1bda146209cbbdf891` |
| M00431 | matado por `i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands` | `0e0d42c02b68c7c0a06312b71d2f23e8df08f6f0f65605c0145708e59fbc34e0` |
| M00432 | matado por `i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands` | `4181926b253ced0c05c9044ff266768dbb7f779d13ff91e0b4279724f89d29d5` |
| M00440 | matado por `i07c_survivor3_nested_partial_result_deferral_keeps_array_shape` | `e6a3817eb7b70bf1e2a961879e41d6973e2400abcfbdf9167beb55fc69fe58d7` |
| M00441 | matado por `i07c_survivor3_deferred_bind_rewrites_pending_join_to_source_operands` | `70b61e5bb9aa6d941a937b07fca0312b2436e82a3c4b2acf735f932681f17fa8` |
| M00443 | equivalente: defer_shape_conflict ya inicializó related_expected con !intrinsic, igual que la asignación eliminada | `95daedf0e3c54a0de1c58925a37d76aca853e653fa96a34b2f024fbeb62e007a` |
| M00447 | matado por `i07c_survivor3_array_join_wraps_pending_operand_shapes` | `e10b7bfdfa72e99688f804e06833da462a893f36d6e6887d5e0db5d8009e4e21` |
| M00453 | matado por `i07c_survivor3_unify_concrete_union_kinds_and_payloads` | `74066d6f1e8f3e9827504a1b7ed86b722612e314539c4bf1e5a62064535575ec` |
| M00463 | sin programa encontrado: cadena DomainNominal: tres programas Len/selección con dominio pendiente no alcanzaron una unión observable distinta | `ed527951ebdbfae357486f5e174056e30afd3a5f72eec720d36966ecdff311ca` |
| M00464 | sin programa encontrado: Bind condicional de nominal: los tres programas Len/selección dieron el mismo resultado | `8c6dadba8b64337272d29e0d9e825b0ec493020ebde5b98ee03055ec98afa6f7` |
| M00482 | sin programa encontrado: DomainNominal con dominio pendiente: no se halló programa cuyo estado Satisfied/Pending cambiara el diagnóstico | `7a813d1b336183618950bd8cef09edcf08dcacd04771debe7880e7e1785738a2` |
| M00483 | sin programa encontrado: DomainNominal con dominio pendiente: no se halló programa cuyo estado Satisfied/Pending cambiara el diagnóstico | `15c72b3e495ac6429d2a8b290d94860f5b1272f810666f1e1d4db122cfc1fd70` |
| M00495 | sin programa encontrado: restauración de visit.tainted tras let condicional: no se halló consumidor posterior que distinga el estado | `8e47f5e23b977f7014d061c06f4f383d940b5b189e491a987084edc4aa8381ba` |
| M00513 | sin programa encontrado: contexto de elemento diferido en comparación: no se halló fuente con diagnóstico distinto | `34e012bc1feccd64bd8249d151a7ba35e04fb5a933be4290400a57a92cce54d1` |
| M00519 | sin programa encontrado: restauración de numeric_domain_owner al cerrar array: no se halló operador posterior afectado | `50f9701066aef7657e4dc8a7a33971f529d0538c918282e373fa6bdeafe79e72` |
| M00537 | sin programa encontrado: limpieza de numeric_domain_owner al acabar RHS binario: no se halló operador posterior afectado | `22084edb3d33d9c12faa79525905a7509f7bb67730d063ad8f90e5e55ad06e6f` |
| M00555 | sin programa encontrado: restauración de numeric_domain_owner ante AnalysisLimit: no se halló resultado posterior distinto | `04bf33c77402d010a816de3738642bf93977e15c39c507e5853b0b0ed2bda8f5` |
| M00561 | sin programa encontrado: propagación de visit.tainted al cerrar payload de unión: no se halló diagnóstico posterior distinto | `71452561170013dfd291c1bfb63041ec27b6a210eb91f52b15eb35de4b67bda7` |
| M00567 | sin programa encontrado: limpieza de numeric_domain_owner tras TryAdd: no se halló operador posterior afectado | `bac400653c10792bf85fc23def4dc7d9a1d80b270ddb891442ffb4fca48eff0c` |
| M00579 | sin programa encontrado: limpieza de numeric_domain_owner al entrar en binario: no se halló operador anidado afectado | `e6ccbbe0bdeabced78a474da37d913a36a44ec181d80240fc475d39c311c8aab` |

## Verificación

Las reejecuciones se hicieron en `C:/nexo-work/surv3/root/disposable/tree`, nunca en el árbol de entrega. `I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, `PATH=$HOME/.cargo/bin:$PATH`, `TEMP=TMP=C:/nexo-work/tmp-surv3`, `CARGO_PROFILE_DEV_DEBUG=CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=1`; una sola carga pesada propia a la vez. El target del verificador, `C:/nexo-work/surv3/root/target-surv3`, ocupó 1,14 GiB. En la copia se ajustó únicamente el script para leer la población congelada y comprobar su hash, evitando regenerarla. No se modificó el script de entrega.

```text
timeout 900 python scripts/i07c_mutaciones.py --base ccf6df1 --unit-prefix check::types::quota_tests::i07c_survivor --ids <ids> --reuse-tree C:/nexo-work/surv3/root/disposable --target-dir C:/nexo-work/surv3/root/target-surv3 --results-copy C:/nexo-work/surv3/root/results-batch3.jsonl --mutation-profile release
```

Se usó `--unit-prefix check::types::quota_tests::i07c_survivor3` para los últimos once IDs y `--integration-test i07c_solver_depth` para M00463 y M00464. La base del último grupo: `0.42 s`; M00537, M00555, M00561, M00567 y M00579: `superviviente (sin diferencia observable)`. La base de integración: `0.68 s`; M00463 y M00464: `superviviente (sin diferencia observable)`. El archivo de resultados de la copia conserva las salidas individuales de los 80 IDs. Cada mutación se aplicó al literal congelado y el verificador comprobó por SHA-256 la restauración del archivo original tras cada ejecución.

Pruebas nuevas con el código correcto, en el árbol de entrega y con `--locked --offline`:

```text
timeout 900 cargo test --locked --offline --lib i07c_survivor3
test result: ok. 31 passed; 0 failed (debug)

timeout 900 cargo test --locked --offline --release --lib i07c_survivor3
test result: ok. 31 passed; 0 failed (release)

cargo fmt --all -- --check
exit code: 0
```

Las llamadas de `nexo run` sueltas se limitaron con `timeout 120`. No hubo mutantes clasificados como detectados por no terminación. No se hizo commit: el coordinador lo gestiona.

En un clon limpio de `HEAD` (`C:/nexo-work/surv3/gate`) se superpuso únicamente el archivo de pruebas y este informe. Allí `sh scripts/validate.sh` terminó con código 0; en esa copia del script cada comando `cargo` tuvo `timeout 900` y la invocación completa `timeout 2400`. Pasaron `fmt`, `clippy --all-targets -D warnings`, tests debug (biblioteca: 1184 pasados, 53 ignorados) y release (biblioteca: 1186 pasados, 54 ignorados), todas las integraciones y RD normativo 152/152 en ambos perfiles. El target ocupó menos de 4 GiB. Tras la última reejecución de M00232 sobre el archivo final, el verificador informó `detectada` y tanto la copia como el árbol de entrega conservaron SHA-256 `bab5d26000161f5375815b9b3206fdcae6ad2ee71b02051dc597f4cca96b2494`.
