# I07c C: regresiones de mutación, lote 4

Población congelada: `docs/validation/i07c-mutations-ccf/population.json`, SHA-256 `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4`. Entrada: `D:/nexo-work/mut-s3/survivors-batch4.json`, 91 IDs. Base de producción S3: `11a70ec`. Solo se añadieron pruebas de integración y código `#[cfg(test)]`; no se cambió producción, ADR ni baseline.

## Rutas y criterio

En una copia desechable se instrumentaron `match_tests`, `bind_match_pattern` y `require_match_pattern_shapes`: el programa oficial con `Some(Some(y))` alcanzó esas rutas durante `check` y `run`. Las pruebas de fuente con array diferido, operador numérico, match inválido, patrón de constructor parcial y aridad, y las pruebas internas de Bind y taint se contrastaron con el código correcto y los mutantes. Para los IDs sin diferencia reproducible no se añadió una prueba de adorno: «sin programa encontrado» describe la búsqueda efectuada y no afirma equivalencia.

Las familias cubiertas son `solve` y obligaciones diferidas; marcos `resume_non_match` (if/while, binarios, llamadas, match); `expr_inner` y proyecciones; cierre de binarios y arrays; patrones de constructor; `branch_accept`, `assert_eq_*`; y emisión IR de match. Las rutas de visita pueden restaurar taint/owner al cerrar el frame, de modo que se comprobó el diagnóstico final además del estado interno. Los mutantes IR M00727/M00730/M00733/M00734 también se probaron con el test de integración existente, sin diferencia observable.

## Resultado por ID

Antes de agregar pruebas se reejecutaron los 91 IDs con los tests existentes de lotes 1-3: 87 en `check::types::quota_tests::i07c_survivor` y cuatro de IR en `i07c_solver_depth`. Solo M00448 fue detectado; los otros 90 continuaron como supervivientes en ese contraste.

| ID | Resultado y motivo | SHA-256 blob mutante |
|---|---|---|
| M00436 | sin programa encontrado: `solve`; las continuaciones de Bind y unión ensayadas no cambiaron el diagnóstico | `80fc1c72c70967019db107035ad8b883927edb428fe4fdd575b6d3c4a68003c8` |
| M00442 | matado por `check::types::quota_tests::i07c_survivor4_deferred_bind_keeps_branch_policy` | `c24f1637c00132b5d3f7e9c0501d11af2124f1da51f189c93c564a5963269c3c` |
| M00448 | matado por `check::types::quota_tests::i07c_survivor3_array_join_wraps_pending_operand_shapes` | `b73947dfd349fc70af2498c565ed8df4ec3916a15894044338b8d6d6728d8ab6` |
| M00462 | sin programa encontrado: `function`; las obligaciones diferidas y los dominios nominales ensayados conservaron el mismo resultado | `e4ccd0d8d4cf52fbeba2ad508fc9915a4ff976b009c09ac62a87875340ead398` |
| M00466 | sin programa encontrado: `function`; las obligaciones diferidas y los dominios nominales ensayados conservaron el mismo resultado | `3cf25a1a15c1fe0a6974cfd269f4c1cafbcfe9f96cf3cc3bdb2f9b5f7b99707d` |
| M00474 | sin programa encontrado: `function`; las obligaciones diferidas y los dominios nominales ensayados conservaron el mismo resultado | `beae5e975e3ab50a5dd03d71cc9c867dc48dea4411f29808939b08281ac4c7c1` |
| M00475 | sin programa encontrado: `function`; las obligaciones diferidas y los dominios nominales ensayados conservaron el mismo resultado | `e089b450a46355bb438d6cabaa872cfc019438c8ff310fc2593fd1a23b30bd79` |
| M00485 | sin programa encontrado: `function`; las obligaciones diferidas y los dominios nominales ensayados conservaron el mismo resultado | `1a638517a5e4d578df2717e84d0898689f2f5d42085c3fec45723414dc09f70a` |
| M00494 | sin programa encontrado: `finish_statement_at_site`; la fuente Len con asignación posterior no cambió el resultado | `d23e9e007f90f3da1cc9c8a739e2bdc0500d918fe820e4a44e33e8d5997b1ced` |
| M00515 | matado por `check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit` | `4e5857d00670801f87f21ece7e26e8fbdc9524a7fbe36c176a81817262ba4fd9` |
| M00516 | matado por `check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit` | `07fe9fe66f938707ab0868cf571bf46cef45390d03e7d7fe80213ba9d827c4ba` |
| M00517 | matado por `check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit` | `4ad387f85cecbfb6602fc5275c9b91b4da447d5bee51804c7a3989f1181c42c4` |
| M00518 | matado por `check::types::quota_tests::i07c_survivor4_invalid_control_condition_taints_visit` | `03ae19189a9966fe7c485ba955536b4ab173db5e926b6d6c1f6c90c3fb3173e0` |
| M00523 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `fb3d562d011b9620437df5376022a5e21d2ea0f4275c273f65b98f9e5e839847` |
| M00527 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `ced4daeda6332bd79068176c12b64aaeb6c053966b2389c956e1e9bf21998171` |
| M00529 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `d051d436d9b5bc84b3dec7758c12caf463b14fdadcdd95f344d04120c4987aa8` |
| M00530 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `5737ae5bf6ceec4e71c953b14633c2f9439ca3ce0236da64c9f9b5d913f11848` |
| M00532 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `04dc25e3bc1dad2f3a8d1333c75ee2ecd72b7dbb185525619638777d9c4095d6` |
| M00533 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `1ae3839f3290f3f59b6f9cfeb9714bd45a16f0e5852f641867606a36c9b71755` |
| M00534 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `19146fa94d054e7ceface5143c76ab40fe6ba74729b0562dc0c96fffcbd0b785` |
| M00535 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `a58048b9f0daf3e1967d944923cd8cce5a06b82ab41e6021c0dcf109f3d8c982` |
| M00536 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `d7fae075d5f95c3cb6517c27394c3e26973e129e4cbffc491c21652ea53b6e54` |
| M00538 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `370dd32b196d95123c52bbbc630599b72bec5b366c2cbc0d36885b92ff1f4958` |
| M00539 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `7818bb1ad2637fab743bdd0188b47e3379c47f7490c3bc9b7eaf40442c4ee6be` |
| M00540 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `ebb7dc277dc2d74610c9aeca1a8c1f767a7adf75fafb7ab16c65770af8a63f9b` |
| M00541 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `78e0aa53c6e6d97d08825e610e96b4d159c0453a423b980dbcbe4c09c1119ba7` |
| M00542 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `2e92974eb71a60a70a5e7675f550a75bc07356b5fa5bf560e3ee12987a885117` |
| M00545 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `6775370ba89b5d1b14aeaa4588794e52163906449134d53c44758562321ef9cf` |
| M00550 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `bdef3cbb069c96c25790bc216d325f42ae213bf2323e32cc2b2a345b3cb74dda` |
| M00551 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `96fb94dbb9fd7604d7d6baa496e988c69fa2572c20a9a0d7d28cfd077c41a0f4` |
| M00552 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `58303beac2271c58a8b4a091b6bfebd84f69c1e9fb55cec55da3e3716dfc9f68` |
| M00553 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `1115af80def7f5167ad20a699872acf178ca6251ce7687443aab7b024068bb17` |
| M00554 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `13f775c8c2be4df925f0db1c712c47ea6a90687e0b4479724257985fb25a6071` |
| M00556 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `ddc3252026622ef7ed4a727faf4a4e25ea0289e56512a898c8f07ed20a713fa4` |
| M00557 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `542fa6992e6ef1c36f758bd10dcc1d0f86ee74015777d90747160dac869a886d` |
| M00558 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `bf1e6302ecccb7246c5ea9fa23bc1da47747d3dc60f5fd7950a16c854e02ee00` |
| M00559 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `a9c45d6d32983d5d7352a7f1351b60edc98f18201f73519e357adcb809a7e587` |
| M00560 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `bedd3107257be6c2afdf096b8ea45a36ba69d1543be45cac1066b4a7ffc2ebe6` |
| M00562 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `508d21dc7c9b73a4f4dd7648e55736af6fa00bcf2157fb7aa163e62ef4f199e3` |
| M00563 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `a8cc628124d4be69b715f0b6470f1161fa488e4ffbfe28848df8540e6f1b6846` |
| M00564 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `cb11324099a90b1ab4b814db85336f318ed7ae21a4891c4cea180496f267356a` |
| M00565 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `f670b171b9d8fd37e24083027fda0ed932f6627e8780b02181ad318f08323371` |
| M00566 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `246d18c1fc015767811753f5cfe3500fa4f314c96e5d70315a3ff23b1f233c15` |
| M00568 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `9ece5833f976a33c9911dd94bf1c2320599c1b30a9e1f350275b0492cb1461f0` |
| M00569 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `12ec819906ad413b83bb2431bcdcb9f3cbdb8356d0dbfeafbde33f738808e1fa` |
| M00570 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `66dc8da4fac13c82006f86420046b28c0fab9f869327633e7d11cfef53451dc0` |
| M00571 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `c2b188f01304ee608fa23e3d7f112a3759af64aeb867aad2ef5d58d8b15d21e2` |
| M00576 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `87edb230ce92b9a3544a1bd34c733bb424c6256f01732f129517407b28bd7ac2` |
| M00577 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `ed3f64ac7e8c19c45ba4065bb08100f43858c72fbf59c1298ea466b19c990c09` |
| M00578 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `6999d420c39e06a63a36e28fa33a863931da314ebab3705060029f297914d59a` |
| M00581 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `fa799ff6e488642e76b3ff81da68a42946d7cfc35d0e8efac5d9e3c69c502e2f` |
| M00588 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `75ea482068a44e5f92d42c67c5e6241703fd9e7fd615888969bdf3dc5fa7ebd2` |
| M00589 | sin programa encontrado: `resume_non_match`; los marcos de control, binarios, llamada, intrínseco y match ensayados no cambiaron resultado ni diagnóstico | `950cb278e4360636968e27300a6d1401f089d08a9469ee4f577ea836cbffa041` |
| M00592 | sin programa encontrado: `check_against_context_one`; las comprobaciones de contexto diferido ensayadas no cambiaron el diagnóstico | `5e4a11ca341ec052afc7c2beec964caa26150221010d16c23f31369536ef0111` |
| M00593 | sin programa encontrado: `check_against_context_one`; las comprobaciones de contexto diferido ensayadas no cambiaron el diagnóstico | `7aa9d74bc0fd912c76676ffb0987c5efebf8d8cec5d456e201222942021056cf` |
| M00603 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `a8a2ce71febd9af9f915adfa843a3667edd4102f192b46899d851c5aad55203a` |
| M00604 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `565ddbb2df805a8fc54a837c514726038b507ba297a2455e51629cb9904c7bea` |
| M00607 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `57195c72f1470f744694c8c8e38b9dfc08d90b8f628f8743845da2db4387ef0a` |
| M00616 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `bd48a5d934311d59fa7c5b543a2261a494f37bcd4cb960cc220b5ab0a62768fd` |
| M00617 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `9419f4d66a006cff26e0dd214400b94a6377e0b0f4b83df69570911f1579d5ce` |
| M00622 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `baeb4d2e8918a1bca861e384de0b8f65c53501c31b1a6fe63f683c9a055a6fb2` |
| M00624 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `46d4f8f2e45a1ae64505b56b80766444f0cead813e77a5406cdd4860b8763615` |
| M00626 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `ed73d97629e1584c062b4d5e25a003c9bbd1c082cfecdec9fc90d37b9ce3e745` |
| M00627 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `b906e97d5a8c20f500280124b8b195024f3fb21b02718d756d3cdd1bf06b4af0` |
| M00628 | sin programa encontrado: `expr_inner`; las expresiones indexadas, con error previo y con contexto pendiente ensayadas no cambiaron el diagnóstico | `45f23026ea2b26d16c752f599225f8b1c427c58b0a17188602a0b83879c0850c` |
| M00642 | sin programa encontrado: `array_accept`; el array con elemento incompatible ensayado conservó el diagnóstico | `c03e918ea40710d97cab8a3aedad0ef2a3e98b2698c61558305273c66f778565` |
| M00643 | sin programa encontrado: `array_accept`; el array con elemento incompatible ensayado conservó el diagnóstico | `fe4d86e3f40bef726afa0899a27105fd3ae0116a8ac141cb8e0f06039a5a8278` |
| M00652 | sin programa encontrado: `finish_deferred_binary_right`; los operandos numéricos diferidos ensayados conservaron el diagnóstico | `cec82b2f3ab75ff1e0119f6c0a91a1aef75329d772ea025d60b8f6db8c1bd33a` |
| M00653 | sin programa encontrado: `finish_deferred_binary_right`; los operandos numéricos diferidos ensayados conservaron el diagnóstico | `c4757a4e2fe9da10303c17af140b84499fa6ff522fbdb92e1cdf6b83aced193c` |
| M00655 | sin programa encontrado: `finish_deferred_binary_right`; los operandos numéricos diferidos ensayados conservaron el diagnóstico | `b0e2d27701f0bc4033197bea434a0f9e72f150a4e09d53339f8032373de6aae3` |
| M00663 | sin programa encontrado: `finish_binary_right`; el binario con error independiente conservó el diagnóstico | `fa19804825a2c82b20c84903ebc8a94301213f2fea1ee33d39ab55c362750d04` |
| M00664 | sin programa encontrado: `finish_binary_right`; el binario con error independiente conservó el diagnóstico | `bef8aa75db58378ee21887ed1adabab4c0511c1b0773a84e7903bdb1a4b22649` |
| M00668 | matado por `partial_constructor_shape_uses_canonical_message` | `3e54e0bddd5458fb15ba0cf73ed74506dfda670e61a559244591347f0bda60d1` |
| M00669 | sin programa encontrado: `require_match_pattern_shapes`; el nombre ajeno a Some/None/Ok/Err se rechaza en nombres antes de esta ruta | `d8074260537d117ac27e2dd7bbde30a1bf00fcb28d0b7f17ba8ef25ea42fd320` |
| M00670 | equivalente: el mapeo previo solo asigna Option a Some/None y Result a Ok/Err; `kind.variant_tag(name)` siempre devuelve Some | `15fed8c8991181cd2c3fa7d7066c0f6725789229b934babecf7453116d0f6bad` |
| M00674 | matado por `unresolved_array_match_does_not_loop_on_option_pattern` (no terminación >60 s; base 0.06 s) | `ef414f3e2897d60e2a36b2c90f28c2758e6e3169a15a3134341bdd9dc4e9b1ab` |
| M00677 | matado por `constructor_arity_errors_do_not_repeat_pattern_walk` (no terminación >60 s; base 0.07 s) | `435a58c6739b42c453b81b467b1a43cc5925565906ff0a89c9f6134d77568da7` |
| M00678 | matado por `constructor_arity_errors_do_not_repeat_pattern_walk` (no terminación >60 s; base 0.07 s) | `b7a7a4af43bef113337d24b0368610c60c37eb5d8bf963fc5a53f4ecd79de5c8` |
| M00701 | sin programa encontrado: `branch_accept`; las ramas de forma pendiente ensayadas conservaron el resultado | `45822e87dac6a3f059901a3ac1f03c6151d879af645cd9b15004c55875f1b201` |
| M00702 | sin programa encontrado: `branch_accept`; las ramas de forma pendiente ensayadas conservaron el resultado | `5271c14c3346cd230f23405aab75c3f8449eff8f1c96ad8c75cad4c8ff4696f0` |
| M00703 | sin programa encontrado: `branch_accept`; las ramas de forma pendiente ensayadas conservaron el resultado | `c3f1f9d853aa6e093eead2893c07754bf2b1fa479b0f69ad3dcf85d10ed60bf5` |
| M00713 | sin programa encontrado: `assert_eq_prepare`; las comparaciones con error previo ensayadas conservaron el diagnóstico | `aac71c7e50e73bfa883f955454cbf8070bf93fd6759b980da194f4cbe508ff87` |
| M00717 | sin programa encontrado: `assert_eq_accept`; las comparaciones con origen y error previo ensayadas conservaron el diagnóstico | `2786859c5b0a02197b12aaa891a42c1e7f236daf7d4e29da3641cb75cc69f94f` |
| M00718 | sin programa encontrado: `assert_eq_accept`; las comparaciones con origen y error previo ensayadas conservaron el diagnóstico | `19233b635aa13dbfcbefb74382beb201777d40de92dd2fc86e6ea108a780046e` |
| M00719 | sin programa encontrado: `assert_eq_accept`; las comparaciones con origen y error previo ensayadas conservaron el diagnóstico | `043100b676da3342dfaa751cd00992019da1b42a3e56872da03f7d6d7572c779` |
| M00720 | sin programa encontrado: `assert_eq_accept`; las comparaciones con origen y error previo ensayadas conservaron el diagnóstico | `0af10f9fde5bdde4e3ae17adf5b4e1212c70408aa143e88ac2bd4b15e6846591` |
| M00721 | sin programa encontrado: `assert_eq_accept`; las comparaciones con origen y error previo ensayadas conservaron el diagnóstico | `498e38e4a51ddb90f979c34a1e35d1343e1fbfcf4f33ae177a108a23ddc7a95b` |
| M00727 | sin programa encontrado: `match_tests`; los matches anidados que llegan a IR conservaron los eventos | `876d24e7f13b8b6eff6a274d4969564fc8e5b0fe3bf991d5b06f2a77f773700b` |
| M00730 | sin programa encontrado: `bind_match_pattern`; los matches anidados que llegan a IR conservaron los eventos | `50659b87a4e7912ba258bc6ac3ccfa7fd2e398325f09ee4728e088d4fa72d795` |
| M00733 | sin programa encontrado: `bind_match_pattern`; los matches anidados que llegan a IR conservaron los eventos | `304fed13ceb0ccf82ceed275b38fb7b816c64dc8e9af7d38d6775aea86e912c0` |
| M00734 | sin programa encontrado: `bind_match_pattern`; los matches anidados que llegan a IR conservaron los eventos | `614097341233e165ab2572dd280160c4f8ce54be4d5434dedaff7fee196a8495` |

Conteo: 10 detectados (9 por pruebas nuevas y M00448 por una prueba previa), 1 equivalente, 80 sin programa encontrado. Los detectados por no terminación, si los hay, son: M00674, M00677, M00678.

## Verificación

El verificador se ejecutó solo en `C:/nexo-work/surv4/repo/disposable/tree`, con `I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, target `C:/nexo-work/surv4/repo/target-surv4`, `--base ccf6df1 --reuse-tree C:/nexo-work/surv4/repo/disposable --mutation-profile release`, `--unit-prefix check::types::quota_tests::i07c_survivor4` o `--integration-test i07c_solver_depth`, y `--ids` por familia. Cada invocación fue protegida por `timeout 900` y `scripts/heavy-lock.py`; el script restaura cada fuente y afirma su SHA-256. En la copia del script se leyó directamente la población congelada y se amplió a 600 s el plazo de compilación fría; no se cambió el verificador de entrega.

Comandos finales del verificador (las otras familias cambian `--ids` y el filtro):

```text
timeout 900 python -B scripts/heavy-lock.py run --session surv4-final-unit -- python -B scripts/i07c_mutaciones.py --base ccf6df1 --unit-prefix check::types::quota_tests::i07c_survivor4 --ids M00442,M00515,M00516,M00517,M00518 --reuse-tree C:/nexo-work/surv4/repo/disposable --target-dir C:/nexo-work/surv4/repo/target-surv4 --results-copy C:/nexo-work/surv4/repo/results-batch4.jsonl --mutation-profile release
Base check::types::quota_tests::i07c_survivor4: 0.08 s
M00442, M00515, M00516, M00517, M00518: detectada
timeout 900 python -B scripts/heavy-lock.py run --session surv4-final-arity -- python -B scripts/i07c_mutaciones.py --base ccf6df1 --integration-test i07c_solver_depth --ids M00677,M00678 --reuse-tree C:/nexo-work/surv4/repo/disposable --target-dir C:/nexo-work/surv4/repo/target-surv4 --results-copy C:/nexo-work/surv4/repo/results-batch4.jsonl --mutation-profile release --mutant-timeout 60
Base i07c_solver_depth: 0.07 s
M00677, M00678: detectada (no termina en 60 s)
```

La repetición aislada de los tres casos de no terminación usó `RUST_TEST_THREADS=1` y dos workers de cargo en la copia del verificador: M00674 tuvo base 0,32 s y mutante >60 s; M00677 y M00678, base 0,37 s y mutante >60 s. La reejecución sobre los archivos finales confirmó bases de 0,06 a 0,07 s y los mismos timeouts.

El código correcto pasó `cargo test --locked --offline --release --lib i07c_survivor4` (2/2), `cargo test --locked --offline --release --test i07c_solver_depth` (16/16), y los mismos comandos en debug (2/2 y 16/16). El test anterior `i07c_survivor3_array_join_wraps_pending_operand_shapes` detectó M00448; no se duplicó.

Restauración SHA-256: `src/check/types.rs` `572dca78c597a56b9f0cf10b676f658ca69afb1e6ed7c057c73b97f22c1fab1e` y `src/ir/lower_match.rs` `29ed46fe5512dfbe6b333fa256873e93701aa10412b0436ea6f96b62a8895f7e`. El SHA-256 de cada blob mutante completo figura en la tabla; se calculó aplicando el literal y reemplazo congelados al archivo final con fin de línea LF, como `Path.write_text` del verificador.

Puerta completa: `sh scripts/validate.sh` en clon limpio `C:/nexo-work/surv4/gate2` terminó con código 0. `fmt` y `clippy --all-targets -D warnings` pasaron; nextest `--profile default` debug: 2070/2070 pasados, 72 omitidos; release: 2072/2072 pasados, 73 omitidos. El test RD `rd_table_matches_check_and_run` pasó en ambos perfiles. El script de la copia envolvió cada comando cargo en `timeout 900`; la invocación completa tuvo `timeout 2400` y el cerrojo pesado. SHA-256 del log `d7ad9baac1f5c53eabbca23f2575bfbf5ebf4c776a3b5a37cd3da068e03a7dc5`; target de la puerta: 2,6 GiB.

La limpieza de `C:/nexo-work/surv4` quedó pendiente: la revisión automática bloqueó `Remove-Item -Recurse` incluso sobre un target con ruta explícita. Los logs y resultados pequeños quedaron copiados en `C:/nexo-work/surv4/artifacts`; no se detuvo ningún proceso ajeno.
