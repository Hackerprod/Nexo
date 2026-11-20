# I07c C: supervivientes de mutación, lote 5

Población congelada: `docs/validation/i07c-mutations-ccf/population.json`, SHA-256 `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4`. Entrada: `D:/nexo-work/mut-s3/survivors-batch5.json`, cuatro sustituciones que coinciden literalmente con esa población. Base de producción S3: `11a70ec`; árbol de entrega: `6c59a15` (incluye el merge de main y los lotes 1–4). No se añadió ninguna prueba: M00432 ya tiene una regresión discriminante, dos mutaciones son equivalentes por invariante local y para M00531 no se encontró un programa discriminante. Añadir una prueba que pase también con el mutante falsearía el resultado.

## Alcance de las rutas

Se instrumentó **solo** `C:/nexo-work/surv5/tree/src/check/types.rs`, una copia desechable. Con `timeout 900 cargo test --locked --offline --test i07c_solver_depth -- --nocapture`, 16 pruebas pasaron y las marcas de M00387/M00432/M00531/M00538 aparecieron 510/0/18/0 veces. Con `timeout 900 cargo test --locked --offline --lib check:: -- --quiet --nocapture`, 242 pruebas pasaron, 2 quedaron ignoradas y las marcas aparecieron 7/1/49.078/19 veces. Por tanto, las cuatro rutas se alcanzan desde la batería de programas fuente o pruebas internas; no se clasificó ninguna como inalcanzable. El archivo instrumentado se restauró por SHA-256 antes de verificar mutantes.

En la misma copia se ejecutó una prueba exploratoria temporal con 1.512 combinaciones de expresiones binarias en sintaxis oficial (`+`, `-`, `==`, `<`, 21 operandos izquierdos y 18 derechos, incluidos dominios pendientes, errores, arrays, constructores y llamadas). Se compararon diagnósticos, tipos inferidos, resultado intermedio de `drive`, taint, número de joins, obligaciones y flags de ocurrencia: **0 diferencias** para M00531 y M00538 frente a la base. Esa prueba exploratoria no se incorporó al árbol de entrega porque no distingue mutantes. Este resultado acota la búsqueda, pero no demuestra equivalencia de M00531.

## Resultado por ID

El SHA-256 de blob mutante se calculó sustituyendo el literal congelado una sola vez en el `src/check/types.rs` del árbol de entrega, con la misma escritura UTF-8/LF que emplea el verificador. Tras cada sustitución se restauraron los bytes originales y se comprobó SHA-256 `572dca78c597a56b9f0cf10b676f658ca69afb1e6ed7c057c73b97f22c1fab1e`.

| ID | Veredicto y fundamento | SHA-256 blob mutante |
|---|---|---|
| M00387 | **equivalente**: `BindUnionFinish` sigue a `BindUnionAfterRequire`. Los descriptores canónicos `Option` y `Result` aportan respectivamente uno o dos pares de payload; cada `Bind` hijo exitoso devuelve `SolverValue::Unit`. Al quitar la asignación final, `outcome` ya es `Unit`; un error toma la salida anterior. | `8276d2dddd81834f1821d1ac45fad0b140d6a70941e25ab6eb97e659cce4290c` |
| M00432 | **matado por** `check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands`, prueba ya versionada en lote 3. Comprueba `related_expected` del join diferido; no se duplicó. | `c7b9c6bf13871b53333a5f5f91034c35b332f7a61f935be8dd4590c60151c8a5` |
| M00538 | **equivalente**: la rama `right_failed` entrega inmediatamente `Ok(Type::Tainted)` a `check_against_context_one`. Esa función marca la expresión `Tainted` y pone `visit.tainted = true` antes de usarlo para el chequeo contextual. La asignación suprimida no cambia el estado posterior ni el diagnóstico; el ensayo de 1.512 casos tampoco dio diferencias. | `370dd32b196d95123c52bbbc630599b72bec5b366c2cbc0d36885b92ff1f4958` |
| M00531 | **sin programa encontrado**: el cambio `&&`→`||` amplía la entrada al marco `BinaryRight` cuando una condición previa la impediría. Las 1.512 combinaciones y la batería `check::` dieron iguales diagnósticos, tipos y estado; no se probó que todos los estados futuros sean equivalentes. | `056c30e48e0eafd04c1e4ea2bed105e441a6fcc43841ec8b527c14cefaeced6c` |

## Verificación dirigida

La copia `C:/nexo-work/surv5` contiene `tree` y `target-surv5`; no se usaron `D:/nexo-work/gate-b`, `gate-r2` ni los targets de la campaña. El verificador **de la copia** lee el archivo de población congelada y comprueba su SHA-256; ese ajuste no se transfirió. `I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, `CARGO_TARGET_DIR=C:/nexo-work/surv5/target-surv5`, `CARGO_BUILD_JOBS=6`, `CARGO_PROFILE_DEV_DEBUG=CARGO_PROFILE_TEST_DEBUG=0` y `TEMP=TMP=C:/nexo-work/tmp-surv5`. Cada comando de test o verificación se envolvió en `timeout 900`, bajo `scripts/heavy-lock.py`; Cargo recibió `--locked --offline` desde el verificador.

```text
timeout 900 python -B scripts/i07c_mutaciones.py --base ccf6df1 --unit-filter check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands --ids M00432 --reuse-tree C:/nexo-work/surv5 --target-dir C:/nexo-work/surv5/target-surv5 --results-copy C:/nexo-work/surv5/results-batch5.jsonl --mutation-profile release
Base check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands: 0.04 s
M00432: detectada (check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands)

timeout 900 python -B scripts/i07c_mutaciones.py --base ccf6df1 --unit-filter check::tests::branch_restores_dependency_taint_after_invalid_return_expression --ids M00531 --reuse-tree C:/nexo-work/surv5 --target-dir C:/nexo-work/surv5/target-surv5 --results-copy C:/nexo-work/surv5/results-batch5.jsonl --mutation-profile release
Base check::tests::branch_restores_dependency_taint_after_invalid_return_expression: 0.05 s
M00531: superviviente (sin diferencia observable)

timeout 900 python -B scripts/i07c_mutaciones.py --base ccf6df1 --unit-prefix check:: --ids M00387,M00531,M00538 --reuse-tree C:/nexo-work/surv5 --target-dir C:/nexo-work/surv5/target-surv5 --results-copy C:/nexo-work/surv5/results-batch5.jsonl --mutation-profile release
Base check::: 67.78 s
M00387: superviviente (sin diferencia observable)
M00531: superviviente (sin diferencia observable)
M00538: superviviente (sin diferencia observable)
```

El parámetro `--profile` del recheck conserva su valor predeterminado `release`; `--mutation-profile release` documenta el perfil solicitado. El verificador restauró `src/check/types.rs` después de cada ID y comparó el SHA-256 con el original. La comprobación independiente de los cuatro reemplazos dio, en cada caso, restauración `572dca78c597a56b9f0cf10b676f658ca69afb1e6ed7c057c73b97f22c1fab1e`.

## Tests de la base

No hay tests nuevos que ejecutar en debug o release. La regresión reutilizada de M00432 pasó en ambos perfiles sobre el código correcto:

```text
timeout 900 cargo test --locked --offline --lib check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands -- --exact
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured

timeout 900 cargo test --locked --offline --release --lib check::types::quota_tests::i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands -- --exact
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
```

La puerta completa se ejecutó en el clon limpio `C:/nexo-work/surv5/gate`, HEAD `6c59a15`, con el mismo target aislado y `timeout 900`:

```text
sh scripts/validate.sh
fmt: PASS
clippy --all-targets -D warnings: PASS
nextest --profile default debug: 2070/2070 PASS, 72 skipped
nextest --profile default release: 2072/2072 PASS, 73 skipped
python -B scripts/i07c_rd_conformidad.py --verify C:/nexo-work/surv5/target-surv5/debug/nexo.exe
RD snapshot differences: 0 (informative)
RD normative: 152/152 check/run pairs
```

SHA-256 del log de la puerta: `f3ab44cdec131e0a87f818cfb89b7e099e1ff64ba6a28dc49f56149809c4ecf3`. El target ocupó 2,36 GiB y el clon siguió limpio. No se modificó código de producción, pruebas existentes, ADR ni baseline. El diferencial del archivo de producción contra HEAD está vacío.
