# Notas de implementación F0-0002 / I03

Estado: validada tras revisión independiente y publicada. Estas notas describen el código presente; no acreditan aprobación de F0 ni cierre del perfil completo.

## Alcance y archivos

`src/check/names.rs` registra todas las firmas antes de recorrer cuerpos, rechaza duplicados de función y parámetro, resuelve locales por ámbito léxico con el preámbulo exterior, informa bindings duplicados y nombres ausentes, y asocia argumentos a parámetros sin reordenar los argumentos escritos. Distingue los siete errores estructurales de NX-E0111, la proyección inválida del intrínseco `drop` y las dos formas básicas de entrada NX-E0112. Las funciones de usuario sombrean los intrínsecos; los parámetros y locales sombrean funciones y preámbulo.

`src/check/types.rs` comprueba anotaciones, parámetros, retornos escritos, asignaciones simples, literales `i64`, arrays vacíos y heterogéneos, índices, dominios de intrínsecos y comparaciones. Conserva tipos estructurales escritos para rechazar igualdad de tipos que contienen recursos y arrays de recursos introducidos por alias. Propaga contexto local a operandos y a los `value:` de ramas simples; los caminos y las continuaciones normales pertenecen a I07. `src/check/tests.rs` y el test interno de `names.rs` fijan diagnósticos, restauración de ámbitos, casos válidos, fuzz, pila pequeña y escalado. `src/check.rs` exporta `check_proposed(file, source, run_entry)`; el `check` legado, `compile` y los comandos `nexo check`/`nexo run` no invocan ese API.

`HANDOFF.md`, `Pending.md` y `RECOVERY.md` conservan sus antecedentes y añaden el estado local del corte. No se cambiaron `docs/baseline/`, los ADR ni el comportamiento del frontend legado.

Base publicada de este trabajo: I01 `d4836cf`, I02a `f073275` e I02b `84d5f53`. Los acuerdos del propietario publicados en `5d351d0`, `c80d8d3` y `b32ea66` son documentación previa, no commits de I03.

## Comprobaciones locales

- L01 emite NX-E0102/type_mismatch por el retorno vacío terminado por LF, con el tipo escrito y el delimitador relacionados. L07 emite NX-E0101/value con los offsets originales de UTF-8, LF y CRLF. Sus positivos completos permanecen en I05 e I04, respectivamente, según §17.2.
- Los negativos independientes de S01–S07, S10, S12–S21, S23–S26 y S29 tienen tests. La entrada sin `main` y la firma con parámetros tienen tests en el API aislado. La tabla de §17.2 asigna el cierre de los positivos y observaciones a I04–I09; aquí no se atribuye ese cierre.
- `Diagnostic::validate` comprueba los diagnósticos del checker, incluidos los generados por 20.000 fuentes mutadas. Una batería aparte construye 8.000 programas válidos con nombres contextuales, sombreado y comparaciones sin diagnóstico. El checker ejecuta las pasadas semánticas en un hilo acotado de 16 MiB; la sonda llama al API desde pilas de 256 y 512 KiB en ocho familias y doce profundidades. La prueba de escalado en release recorre 16 familias por defecto y 24 con `NEXO_FULL_SCALE`, de 256 KiB a 1 MiB.
- Los tipos inferidos admiten exactamente 128 envoltorios Array y rechazan el 129 con `array_type_depth`; 128 arrays sobre un Result escrito alcanzan `value_type_depth` al envoltorio 129. El test fija `limit`, `observed`, etapa, operación y span de la expresión constructora.
- Las regresiones vigilan el primario y el salto que terminó un retorno vacío, los spans escritos de anotaciones acabadas en `>`, la acumulación de errores independientes de registro y cuerpos, el orden names → arguments → types, la asociación de parámetros ausentes, el origen del primer elemento en arrays heterogéneos, el tipo de condición de `if` expresión, los retornos unitarios, la resolución de ramas, la supresión de cascadas dependientes y la restauración de ámbitos y contexto de tipos.

## Lagunas y límites explícitos

- §13.15 exige relacionar la etiqueta de la primera asociación en NX-E0111/duplicate_argument. Si la primera asociación fue posicional, no existe etiqueta. `adr_gap_duplicate_named_after_positional_relates_first_value` fija que el `context` actual señale el valor posicional previo; no añade una regla al ADR.
- §3.3 reserva `move` y `async`: `move_and_async_are_reserved_names` comprueba su rechazo como bindings y los positivos `move_value`/`async_value`. No es una laguna del ADR. El generador de 8.000 programas válidos usa los nombres contextuales y de preámbulo aceptados.
- §17.2 sitúa el cierre completo de NX-E0112 y de Result en I08. El API aislado distingue la firma `main` básica y reconoce algunos tipos `Result` escritos y constructores con contexto, pero no implementa inferencia completa de constructores anidados, propagación ni cierre de entrada. `adr_gap_try_result_type_is_deferred_until_result_semantics` fija que `?` conserva un tipo diferido sin producir un falso NX-E0102 en I03. La semántica de `?` pertenece a I08; el control por caminos y las ramas sin valor pertenecen a I07.
- No se ejecutan llamadas ni comparaciones; el orden escrito se conserva en la asociación y comprobación de argumentos. No se acreditan copias, préstamos, permisos, efectos, limpieza, IR, intérprete, desbordamientos de ejecución, análisis de caminos ni presupuestos completos de I10. NX-E0105, NX-E0201, NX-E1105 y NX-E1317 permanecen en sus etapas posteriores. Las comparaciones de arrays y Result solo comprueban dominio y tipos, sin ejecutar igualdad estructural.
- §17.2 cierra S08 en I06, S11 y B01–B09 en I07, S09/S20/S21 y los positivos de entrada en I08, PP06 en I09; S22/S27/S28 son negativos de `parse` ya delegados al parser y su cierre completo está en I04. S29 requiere destrucción para su positivo completo en I05. Las demás filas de §14 mantienen los cierres precisos que fija §17.2.

## Reproducción

Ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh` desde la raíz con la cadena Rust fijada. La revisión independiente debe usar oráculos propios y comparar `check`/`run` con HEAD. Los resultados locales no sustituyen esa revisión.


## Precisión de diagnósticos y límites de I03

- §13.9: `check_proposed` devuelve una lista ordenada de diagnósticos semánticos. Registro y resolución de nombres, asociación de argumentos y tipos corren en etapas separadas; una etapa con errores detiene las siguientes. Se preservan sitios independientes y se omiten descendientes de la llamada inválida.
- §§13.15 y 14.8: L01 usa el token `return` como primario y el primer salto LF/CR/CRLF que lo terminó como `delimiter`, incluso tras espacios, tabulaciones y comentario. El retorno vacío no tiene expresión `found_type`; `return;` no añade `delimiter`. L07 conserva los spans de bytes. I03 cierra solo sus negativos; los positivos completos quedan en las etapas asignadas por §17.2.
- §6.2: una rama que termina en `return` no contribuye tipo a la unión, incluso anidada. El análisis completo de caminos y la emisión de `missing_branch_value` siguen en I07. Si ninguna rama tiene continuación normal, el binding se marca como divergente; los usos posteriores aceptan el tipo esperado sin exponer `()` como tipo encontrado.
- §13.15: `else if` compara cada continuación con la primera continuación normal y relaciona su operando. Los spans de anotación y operador proceden de tokens sin trivia final.
- §8.4: etiquetas y resultados escritos de las primitivas se registran en una sola tabla; las restricciones de dominio se comprueban en tipos. `len` usa `a`. El uso del preámbulo como valor recibe `NX-E0006/function_value` en nombres.
- La sonda de pila cubre profundidades 1, 2, 4, 7, 8, 15, 16, 31, 32, 63, 126 y 127. El parser puede rechazar alguna forma 127 por su propio presupuesto; la sonda exige que las formas aceptadas alcancen semántica sin aborto y que cualquier diagnóstico sea válido.
- `cargo test --locked --offline --lib -- --list` enumera 217 tests en debug; con `--release`, 219. La ejecución completa de `sh scripts/validate.sh` terminó sin fallos; la suite release tardó 131,21 s, dominada por `frontend::parse::tests::parser_token_patterns_scale_with_source_length`. La medición completa con `NEXO_FULL_SCALE` recorrió 24 familias de 256 KiB a 1 MiB, con razones entre 3,89 y 4,66 y 59,15 s de duración.
- La duración de la suite completa se consigna con la ejecución actual. La prueba preexistente `frontend::parse::tests::parser_token_patterns_scale_with_source_length` domina el tiempo en release; no se alteró ese frontend ni se rebajó la prueba.
- El parser todavía decide `resource_array` al reconocer `mock_open` por nombre, antes de resolver si está sombreado. Las guardas defensivas de profundidad del checker son inalcanzables para la familia de bloques aceptada por el parser y sus diagnósticos usan los contadores nominales 1.000.000 y 1.000.001, no pasos medidos. `Ok(Ok(1))` carece de inferencia completa en este corte. El mensaje textual de los errores semánticos es genérico, aunque sus campos estructurados distinguen las causas. En arrays heterogéneos con anotación, `expected_type` puede describir el elemento mientras el span relacionado abarca la anotación completa; la precisión de ese contexto queda abierta. El tipo de `?` se difiere hasta I08.
- §13.9: `adr_gap_same_site_errors_and_dependent_binding_are_not_cascaded` fija la interpretación local de cardinalidad: en `f(true,true)` solo se informa el primer argumento; `zz(yy)` no analiza `yy`; `if bad {bad2}` no analiza el cuerpo; y un error que depende solo de una anotación inválida no se emite de nuevo en el uso posterior.

## Lagunas del ADR fijadas como comportamiento actual

- §4.7 y §13.15: `adr_gap_array_element_mismatch_uses_element_primary_and_whole_annotation_context` fija el elemento como primario y la anotación completa como contexto del tipo esperado.
- §8.4 y §13.17: `adr_gap_primitive_domain_primary_is_first_offending_argument` fija el primer argumento fuera del dominio como primario.
- §4.3 y §13.15: `adr_gap_inout_to_ordinary_parameter_uses_place_primary` fija el lugar del argumento como primario.
- §4.2 y §13.15: `adr_gap_non_name_call_target_uses_whole_target_span` fija el destino completo como primario.
- §4.1 y §8.4: `adr_gap_result_constructor_payload_label_is_v` fija la etiqueta actual del payload.
- §4.7 y §13.17: `adr_gap_empty_array_without_array_context_reports_inference_error` fija NX-E0103 cuando falta contexto de elemento.
- §4.5 y §13.15: `adr_gap_grouped_absent_target_keeps_call_target_use_kind` fija `call_target` para un destino agrupado no declarado.
- §13.15 y §14.8: `adr_gap_explicit_empty_return_uses_keyword_primary` fija el primario de `return;`; L01 con salto conserva el primario publicado.
- §11: `adr_gap_parser_depth_bounds_semantic_guard_for_accepted_blocks` fija que la familia de bloques aceptada no alcanza la guarda defensiva del checker. Los retornos vacíos con CR suelto se comprueban junto con LF y CRLF.
