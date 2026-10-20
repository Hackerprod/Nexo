# Notas de implementación F0-0002 / I02a

Estado: validada tras revisión independiente y publicada en main. No aprueba F0 ni F0-0002 completo.

## Alcance

`src/frontend/lex.rs` aporta `frontend::lex::lex(file, source)`, tokens públicos con grafía y spans `[start,end)` en bytes UTF-8, y `Kind` para nombres, reservadas, enteros, decimal, cadenas, símbolos, saltos y EOF. El módulo se mantiene separado de `src/syntax.rs`; `compile`, `nexo check` y `nexo run` conservan su ruta anterior.

Los símbolos reconocidos son la transcripción cerrada de F0-0002 §3.2, con coincidencia máxima. Los nombres usan ASCII, las palabras reservadas son las tres listas cerradas de §3.3, el decimal sigue §3.2 y el signo menos queda como símbolo separado conforme §3.5. Comentarios `//` omiten su contenido pero emiten el salto posterior como token de trivia; CRLF se conserva en un único token `Newline` de dos bytes.

Los caracteres no reconocidos y las cadenas incompletas emiten NX-E0001 mediante los campos de §13.14 y se validan con `diagnostic::Diagnostic::validate`. Los negativos LX01–LX05 se prueban en `src/frontend/lex.rs`; cada caso comprueba código, subcausa nula, etapa/operación, span, token inválido, contexto, categoría esperada, prefijo reconocido y positivo pareado con `+`.

## Ambigüedades y límites del ADR

- §3.2 solo dice “espacios”, sin cerrar el conjunto de trivia. El lexer actual usa `char::is_whitespace`; por eso tabulador y U+00A0 se consumen como trivia. `adr_gap_whitespace_includes_tab_and_nonbreaking_space` fija ese comportamiento pendiente de decisión del ADR.
- §3 no fija qué hacer cuando un entero va seguido inmediatamente de letras. Hoy `1a` produce `Integer("1")` seguido de `Name("a")`; `adr_gap_digit_letter_boundary_is_integer_then_name` registra esa laguna.
- §3 no fija si una cadena puede cruzar saltos de línea ni la política para una cadena sin cierre antes de LF. Hoy el lexer rechaza la cadena al llegar a LF: para `"a\nb"` NX-E0001 cubre solo `"a` en `[0,2)`, sin incluir LF. `adr_gap_unclosed_string_stops_before_lf` lo fija como comportamiento observado.
- §3.2 enumera cadenas entre comillas dobles como categoría tokenizable y §2.2 las declara diferidas, pero no fija escapes, multilinea ni una gramática interna. El lexer conserva la grafía, permite que `\\` proteja el siguiente carácter para localizar la comilla de cierre, y no decodifica escapes. Esto es una decisión de delimitación léxica, no una afirmación sobre semántica de cadenas; requiere revisión del validador.
- §13.14 describe el span para “lexema comenzado pero incompleto” sin enumerar cuáles lexemas pueden ser incompletos en este paso. Se aplica a comilla sin cierre desde la comilla inicial hasta el punto de fallo, incluido EOF. No se inventan subcausas: NX-E0001 no lleva subcausa.
- §3.1 llama trivia a los saltos, pero también exige conservar su información para terminadores; el token `Newline` es una representación interna para el parser futuro, no un token terminal emitible del lenguaje.
- La distinción entre tokens léxicos de caracteres ASCII y Unicode se aplica literalmente desde §3.3: solo identificadores ASCII; un carácter Unicode fuera de cadena/comentario es un único token inválido con span de todos sus bytes UTF-8.
- §13.8 define las coordenadas de línea contando LF. El lexer conserva CR, LF y CRLF como saltos léxicos (con CRLF en un solo token); un `\r` aislado también se emite como `Newline`, pero no incrementa por sí solo el número de línea textual. Este límite de coordenadas pertenece a la presentación y queda registrado sin cambiar el lexer.

## Comprobaciones

- Tests unitarios del módulo: contrato NX-E0001 con campos completos; LX01–LX05 negativos/positivos con secuencias completas; inventario y coincidencia máxima de símbolos; listas de reservadas y nombres ordinarios con secuencias completas; casos de decimal/rango/punto, `10.25`, `007`, `9`, `1.`, `1.x`, `a//c\r\nb`, `a_b`, `result_from` y Unicode; K01; spans de identificadores, comentario Unicode, CRLF/LF, cadena y EOF. La propiedad determinista comprueba 5.500 fuentes generadas a partir de átomos y bytes, además de truncamientos UTF-8 en el límite de decodificación; el fuzz sin pánicos cubre 20.000 secuencias de bytes y truncamientos.
- Las expectativas de LX01–LX05 proceden de §14.7: fuente base `fn main(){let x=1…2;}`, span `[17,18)`, `recognized_prefix [0,17)`, `invalid`, lexical context y `valid_token`.
- La suite previa del frontend actual, CLI, corpus e I01 corre también con `cargo test --locked --offline` y por `scripts/validate.sh`. `scripts/compare-head.sh` compara las salidas actuales de `check`/`run` con HEAD.

## Fuera de I02a

No se implementa parser, AST de F0-0002, terminadores insertados, recuperación sintáctica, impresor canónico, exclusiones parser-contextuales, reconocimiento contextual de `value`, ejecución ni semántica de literales. I02a no cierra L01–L07 ni el cierre completo de LX01–LX05 que §17.2 asigna a I04; implementa el compromiso léxico y los negativos solicitados en §17 para I02a.
