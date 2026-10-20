# Notas de implementación F0-0002 / I02b

Estado: validada tras revisión independiente y publicada en main. Estas notas describen el código presente; no aprueban F0 ni F0-0002 completo.

## Archivos

- `src/frontend/parse.rs`: AST con spans de bytes UTF-8, parser de funciones, tipos, bloques, sentencias, expresiones y argumentos; terminadores, diagnósticos y reconocedores de alcance.
- `src/frontend/parse/tests.rs`: pruebas unitarias del parser, con los mismos nombres que antes del traslado mecánico.
- `src/frontend/print.rs`: impresor canónico del AST aceptado.
- `src/frontend/mod.rs`: exporta `parse` y `print` junto con el lexer existente.
- `HANDOFF.md`, `Pending.md`, `RECOVERY.md`: estado de I02b añadido, sin retirar los antecedentes de I02a.
- `docs/validation/F0-0002-I02b-implementation-notes.md`: esta trazabilidad y límites.

`compile`, `check` y `run` siguen usando `syntax.rs`. El frontend nuevo no se llama desde ellos.

## Afirmaciones comprobables del ADR y cobertura en tests

La lista siguiente cubre las afirmaciones de §3, §3.1–§3.6, §4 (solo sintaxis de tipos y firmas), §10, §11, §13 asignado a parse, §14 de parse y §17. Las cláusulas de emisión y semántica que corresponden a pasos posteriores se enumeran al final como límites.

| Afirmación explícita del ADR | Test de secuencia/árbol completo |
|---|---|
| `Program` permite varias funciones y firma adelantada; `Name` no reserva `value` | `parses_multiple_signatures_written_types_and_nested_expressions`; `contextual_value_name_stays_an_ordinary_identifier` |
| Parámetros aceptan `inout`/`consume`, varios tipos y retorno escrito | `parses_multiple_signatures_written_types_and_nested_expressions`; `canonical_print_parse_is_idempotent_and_normalizes_terminators` |
| Tipos cerrados `i64`, `bool`, `()`, recursos, `Array<T>`, `Result<T,E>` | `type_close_token_ge_splits_only_in_type_context`; `parses_multiple_signatures_written_types_and_nested_expressions`; `parse_stage_feature_fixtures_keep_their_published_ids` (Option y rutas no soportadas) |
| Binding, asignación, return, value, if/else, while, bloque y expresión descartada están en el subconjunto | `parses_multiple_signatures_written_types_and_nested_expressions`; `return_and_value_colon_newline_end_empty_operands`; `canonical_print_parse_is_idempotent_and_normalizes_terminators` |
| `value:` se reconoce solo como marcador contextual; `value` sigue nombre ordinario | `contextual_value_name_stays_an_ordinary_identifier`; `return_and_value_colon_newline_end_empty_operands` |
| `(`, `[`, `?`, `+`, `-`, `<`, `==` y coma abierta continúan; el nombre de función puede continuar con `(` tras salto | `expression_newline_continuations_do_not_insert_a_terminator`; `canonical_print_parse_is_idempotent_and_normalizes_terminators`; negativos de operador en `excluded_binary_operator_is_not_repaired_as_terminator` |
| Salto tras `return` y `value:` produce operando vacío incluso antes de `(` | `return_and_value_colon_newline_end_empty_operands` |
| `NX-E0005` señala token/rango EOF, `expected_tokens=[";"]`, `canonical_insertion`, prefijo, expresión previa y reparación insertable | `missing_terminator_reports_literal_fields_and_repair`; el test valida el objeto mediante `Diagnostic::validate` |
| `NX-E0002` tiene lista exacta para nombre reservado `match` | `reserved_match_in_binding_name_is_exact_syntax_diagnostic` (S22, spans byte a byte) |
| `NX-E0002` distingue Place inválido para `inout` y `consume` | `place_prefix_negatives_report_the_published_contexts_and_spans` (S30/S31, campos y spans literales) |
| Operadores `*`, `!=`, `>`, `<=`, `>=` no insertan terminador y exigen `+ - < ==` | `excluded_binary_operator_is_not_repaired_as_terminator` |
| `>=` se separa en `>` y `=` solo al cerrar un tipo | `type_close_token_ge_splits_only_in_type_context` |
| Características F01–F33 reconocidas con `NX-E0006`, `subcause=null` y feature ID exacto | `parse_stage_feature_fixtures_keep_their_published_ids`; `f03_f04_and_option_are_recognized_with_their_feature_ids` |
| F03 decimal y F04 rango conservan sus spans de §14.6; Option conserva su ID | `f03_f04_and_option_are_recognized_with_their_feature_ids` |
| K02 `fn main() ;` es error de sufijo con `->`, `with`, `{` esperados | `migrated_k02_signature_suffix_has_exact_expected_tokens` |
| Impresión usa terminadores explícitos y LF; parse–print–parse es idempotente | `canonical_print_parse_is_idempotent_and_normalizes_terminators` |
| `-` es token léxico separado; `/`, `%`, `!`, `&`, `|`, `/*...*/` no se aceptan léxicamente | Tests I02a `minus_is_always_a_separate_symbol_token`, `block_comment_syntax_is_rejected_at_its_first_slash`, `complete_symbol_inventory_and_longest_match_are_tokenized`; parser reutiliza `lex` |
| Símbolos y reservadas son inventario cerrado; `value` y los demás nombres contextuales no son reservados | Tests I02a `reserved_word_classes_match_the_closed_adr_lists`, `underscores_and_contextual_names_are_single_name_tokens`; además `contextual_value_name_stays_an_ordinary_identifier` |
| `NX-E0002` filas §13.14 para `fn`, parámetros, bloque, inicializador, signo, array y basura final | `published_first_rows_have_exact_context_tokens_and_byte_spans` comprueba los siete campos y spans literales enumerados en la tabla de §13.14. |
| LF/CRLF en L01–L07 conserva los resultados asignados a parse | `return_and_value_colon_newline_end_empty_operands`, `contextual_value_name_stays_an_ordinary_identifier`, `expression_newline_continuations_do_not_insert_a_terminator`; los árboles y offsets L01–L07 se comprueban en pruebas específicas. |

Los EX01–EX04 se usaron como lecturas de gramática, no como positivos completos: el parser acepta la porción cubierta por §3 y reconoce exclusiones explícitas como `use`, atributo, `struct`, `for`, `yield`, flotantes y rangos. Las reglas de aceptación de sus bibliotecas no se atribuyen a este AST.

## Sintaxis y diagnósticos implementados

La única producción `WithClause` de D11 §5.2 se modela como lista simple o mapa de operaciones. Para una función ordinaria se aceptan restricciones y mapas con `call`; `with_clause_w01_to_w07_and_accepted_forms` comprueba formas inválidas NX-E0417 y positivos pareados. `no_heap_alloc` sigue fuera del perfil con NX-E0006. Las cotas invocables y `Future<T>` no son tipos aceptados por §3: `missing_with_clause` y `operation_list_required` requieren fases posteriores.

`branch_value_negatives_match_b04_b05_b06_and_l02` comprueba `invalid_value_target`, `missing_value_operand` y `non_final_branch_value` con código, subcausa, etapa, operación, span y ubicación relacionada. L02 usa LF, span `[29,35)`, delimitador `[35,36)` y positivo agrupado. `value:` solo es marcador al inicio de sentencia con `:` en la misma línea; `value` conserva Name en las demás posiciones.

`published_feature_rows_have_exact_spans_and_literal_repairs` cubre F01–F33 con NX-E0006, ID, etapa, operación, spans, campos sintácticos vacíos y positivo reparado. `excluded_operators_keep_diagnostic_across_line_breaks` cubre los cinco operadores excluidos con LF, CRLF y forma contigua. `canonical_printer_preserves_every_ast_field_except_spans` compara el AST completo sin spans tras impresión y nuevo parseo, además de idempotencia del texto.

El parser elimina la trivia de salto del vector de tokens, conserva sus bytes originales y decide cada terminador en `statement`/`terminator` después de reconocer la expresión de esa sentencia. Cada descenso de `statement` guarda y restaura el inicio real del primer token, incluso en bloques anidados: los operadores excluidos usan ese inicio para `recognized_prefix`, también tras LF, CRLF, CR y comentarios. `excluded_operator_prefix_starts_at_active_statement_across_terminators_and_blocks` comprueba 5.000 programas generados con cinco operadores sintácticos, dos características excluidas, cinco contextos de sentencia y bloques `if`/`while`/`else`. Las cabeceras de función y los tipos no reciben `;` virtuales; los bloques dentro de una expresión tienen sus propias sentencias. Los FIRST de estados comprometidos se transcriben en constantes y se emplean para EOF y para token erróneo. `first_state_is_independent_of_the_offending_token` prueba 55 prefijos de estados con 100 rellenos de espacios y 32 tokens candidatos: 176.000 fuentes con token erróneo, más 5.500 fuentes EOF. Incluye los estados internos de `OperationList` y `OperationLimit`, restricciones simples, firmas, tipos y expresiones. `first_consistency_across_nested_grammar_positions` recorre además las fronteras de cuatro programas con tipos, mapas `with`, Place, llamadas y ramas anidadas; `erroneous_tokens_use_literal_first_sets_of_committed_states` fija los conjuntos publicados. La enumeración exhaustiva de todas las posiciones de la gramática permanece pendiente.

`TryExpr = AccessExpr ["?"]` admite un único `?` después del acceso completo, incluidos `consume` e `inout`. `repeated_mode_prefix_is_rejected_before_recursive_descent` comprueba el rechazo inmediato de prefijos repetidos. El reconocimiento de `function_value` se realiza en posiciones de valor; los locales y parámetros sombrean funciones según §4.1 y la fila F09 de §14.6. `non_unit_return` recorre `while`, bloques y ramas `else if`/`else` en orden fuente. `tests/canonical_print.rs` ejercita el API público del impresor indicado por §10 y compara estructura sin spans e idempotencia en 24 programas con `if` en posición de sentencia.

Las fixtures L01–L07 tienen pruebas de parseo y offsets en `l01_l03_l04_have_published_parse_trees_and_byte_spans`, `branch_value_negatives_match_b04_b05_b06_and_l02` y `l05_l06_l07_keep_unit_discard_and_original_utf8_offsets`. `grammar_truncations_cover_nested_production_boundaries`, `with_clause_truncations_use_d11_inner_productions` y las pruebas de terminadores comprueban FIRST, EOF, prefijo, token, reparación y CRLF en los casos nombrados.

## Robustez y presupuestos

El API `parse` comprueba `source_bytes` antes del léxico. `syntax_nesting` cuenta entradas lógicas activas de `Block`, `Expr` y `Type`, sin cobrar la precedencia del operando derecho; `expression_tree_depth` usa altura compuesta al construir nodos; `function_count` rechaza la primera declaración que supera 1.024. Todos emiten NX-E0003 con `budget_kind`, `limit`, `observed` y ubicación del extracto §13.13. Cada parseo usa un hilo ancho de 16 MiB virtual, reutilizado por hilo llamador; no hay preconteo heurístico de delimitadores ni de subcadenas `if`. Si no se puede crear o contactar con el hilo, el API devuelve NX-E0003 de `syntax_nesting` antes de descender, sin panic. `hostile_chains_return_diagnostics_on_a_small_stack` llama al API desde una pila de 2 MB con 10.000, 200.000 y 1.000.000 elementos por familia. `parser_small_stack_probe_survives_nested_forms` ejecuta un proceso hijo por caso para que un desbordamiento de pila de Windows no aborte la suite principal: cubre 1, 2, 4, 7, 8, 15, 16, 31, 32, 33, 63, 126 y 127 niveles de paréntesis, arrays, bloques, tipos `Array`, combinaciones y condiciones `if`, desde hilos de 256 y 512 KiB y el hilo principal del subproceso. `structural_budgets_have_literal_boundaries_and_diagnostic_fields` y `expression_height_counts_composition_without_precedence_entries` comprueban fronteras y campos exactos. `adversarial_inputs_preserve_parser_diagnostic_validity` valida todos los diagnósticos emitidos por las entradas generadas.

`lexer_comment_scan_scales_with_source_bytes` mide el léxico sobre comentarios largos. `parser_token_patterns_scale_with_source_length` solo se compila en release; no corre en debug. Mide 24 familias en 64, 128, 256, 512 y 1.024 KiB mediante la mediana de cinco mínimos, cada uno obtenido de dos ejecuciones. Exige razón 256 KiB→1 MiB ≤ 8 y cada duplicación ≤ 3,5; un cuadrático con razón 22,9 seguiría rechazado. La medición completa duró 129,30 s y las razones de cuatro veces estuvieron entre 4,07 y 4,45. Usa el API público, incluido el hilo ancho reutilizado; el máximo de cada fuente respeta `source_bytes`. `parser_small_source_worker_cost_is_measured` mide 20.000 fuentes pequeñas y comprueba que se cree a lo sumo un hilo en la secuencia. Ejecutar con `cargo test --locked --offline --release --lib parser_token_patterns_scale_with_source_length -- --nocapture`.

## Límites y trabajo pendiente

La enumeración de todas las posiciones de FIRST y de la matriz producción×contexto de terminadores continúa pendiente: `PlaceRoot` recursivo, posiciones de listas/mapas/operaciones/restricciones de `WithClause`, alternativas inválidas de `Statement`/`Program`, cada continuación `(`, `[`, `?`, `+`, `-`, `<`, `==` y coma por producción aplicable, y no inserción estructural LF/CRLF/CR/EOF. Las pruebas actuales cubren casos explícitos, sin agotar el producto de posiciones.

Los doce tests `adr_gap_*` fijan el comportamiento actual en zonas donde el ADR no publica una fila diagnóstica completa:

- `adr_gap_whitespace_includes_tab_and_nonbreaking_space`: clasificación léxica de tabulador y espacio no separable.
- `adr_gap_digit_letter_boundary_is_integer_then_name`: separación de número seguido de letra.
- `adr_gap_unclosed_string_stops_before_lf`: fin de cadena sin cerrar ante LF.
- `adr_gap_incomplete_control_and_postfix_contexts_keep_current_result`: contextos incompletos de `while`, `if`, índice, argumento y prefijos `consume`/`inout`.
- `adr_gap_completed_if_inside_delimiters_keeps_current_terminator`: continuación tras un `if` completo dentro de delimitadores.
- `adr_gap_signed_name_minus_keeps_current_token_and_prefix`: `-` seguido de Name conserva token y prefijo actuales.
- `adr_gap_committed_rhs_first_preserves_current_inner_productions`: producciones internas de RHS incompleto en asignación, suma y comparación.
- `adr_gap_type_unit_terminal_and_lexical_priority_remain_explicit`: terminal `()` de `Type` y prioridad entre error léxico y alcance.
- `adr_gap_branch_marker_and_scope_repairs_keep_current_boundaries`: E1302 frente a E0005 en `value:1}`, residuo `;` en F26 y espacios restantes en F06/F10/F16–F19/F27/F28/F32.
- `adr_gap_nullable_continuations_and_else_if_depth_keep_current_results`: continuaciones anulables y cadena `else if` de más de 128 niveles como `expression_tree_depth`.
- `adr_gap_nonterminating_subexpression_follow_keeps_current_result`: §3 y §13.14 dejan sin fila exacta el seguidor de una condición completa `if a<1`; EOF espera `grammar.Block`/`{`, mientras un segundo `<` o `==` informa `grammar.CompareExpr`/`;`.
- `adr_gap_function_value_and_later_syntax_error_priority_remains_explicit`: §3, §4.1, §13.14 y F09 de §14.6 no fijan la prioridad de `function_value` en etapa `names` frente a sintaxis posterior en la misma sentencia. `let x=f<1<2;` y `let x=f*2;` producen NX-E0002; `let x=f 1;` y `g(f` producen NX-E0006; `let x=(f` produce NX-E0002 `grammar.Primary`.

Los tests fijan el comportamiento actual sin añadir reglas al ADR. El reconocimiento local de `resource_array` depende del constructor intrínseco directo; la propagación de tipos por bindings y alias corresponde a un paso posterior. La detección de `function_value` queda limitada a las posiciones de valor modeladas aquí; su análisis semántico completo corresponde a I03. La reparación NX-E0005 con trivia intermedia se inserta al final de la expresión; los comentarios `//` seguidos de LF/CRLF terminan la sentencia. El sangrado del impresor sigue como límite conocido. El caso `if f({\n}) {}\n x;` queda descartado: `Argument` y `Primary` de §3 no admiten `Block`.

La resolución de nombres, tipos semánticos, propiedad, préstamos, efectos, IR, lowering, intérprete y conexión del frontend propuesto a `compile`/`check`/`run` corresponden a pasos posteriores. Las reglas de emisión asociadas no se atribuyen al parser. La recuperación múltiple de errores tampoco está modelada en el AST.

I01 está validada tras revisión independiente y publicada en `d4836cf`; I02a está validada tras revisión independiente y publicada en `f073275`; I02b está validada tras revisión independiente y publicada en main. Las puertas generales de F0 y la aprobación del propietario siguen pendientes.

## Reproducción

`cargo test --locked --offline --lib -- --list` enumera 127 pruebas unitarias en debug; la configuración release añade la medición de escalado y ejecuta 128. El archivo `tests/canonical_print.rs` tiene dos pruebas; las suites CLI, corpus e I01 conservan sus tamaños de 3, 19 y 37 casos. Las comprobaciones locales no sustituyen la revisión independiente de I02b.
