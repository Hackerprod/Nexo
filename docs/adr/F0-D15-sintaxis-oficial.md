# F0-D15 — Sintaxis oficial de Nexo

## 1. Decisión, autoridad y alcance

Fecha: 2026-10-22. Documento normativo de superficie; su existencia no acredita implementación ni mediciones. La decisión definitiva del propietario de esta fecha adopta **B de F0-D14 §5 como única sintaxis oficial**. A y C quedan descartadas. No habrá selector de dialecto, compatibilidad, detección heurística ni dos frontends de usuario. Al implementar esta decisión se elimina la aceptación de A, y se elimina físicamente el prototipo heredado en ese mismo corte (§7). La prueba de agentes es control de calidad posterior: sus resultados pueden exigir corregir B, pero no vuelven a elegir entre A, B y C.

Esta decisión sustituye los párrafos de selección, competición, convivencia temporal y conversor distribuido de F0-D14 §§9–12 y el estado de «candidata a evaluar» de F0_ACUERDOS §9.6. Conserva los criterios de coherencia, lectura, convención visual y medición de esa sección y su aclaración del propietario de 2026-10-20. No se modifica `docs/baseline/` ni se crea una revisión 0.3.

Referencias abreviadas usadas en todo este documento:

| Sigla | Fuente y autoridad pertinente |
|---|---|
| R | [Roadmap rev 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), §2, D01, D02, D07, D08, D10, EX01–EX04 y puertas F0–F7. |
| AC | [F0_ACUERDOS](../F0_ACUERDOS.md), §§1, 6, 7 y 9.6. |
| A1 | [F0-0001](F0-0001-first-slice.md), Gramática del subconjunto, Tipos y acceso, Evaluación y limpieza. |
| A2 | [F0-0002](F0-0002-second-slice.md), §§2–8, 11–15 y 17. |
| D11 | [Closures](F0-D11-closures.md), §§1–5.3 y 6–10. |
| D12 | [Async](F0-D12-async.md), §§1–10. |
| D13 | [Expresiones y módulos](F0-D13-expresiones-modulos.md), §§1–12 y G.1–G.3. |
| D14 | [Sintaxis de superficie](F0-D14-sintaxis-superficie.md), §§2, 3, 5, 6.2, 7–10. |
| EXT | [Propuesta externa](../proposals/sintaxis-candidata-externa-1.md), §§1–22; se incorpora únicamente lo especificado aquí. |
| COD | [Catálogo único integrado](F0-CODIGOS.md), Registro canónico, Precisiones de F0-0002, Feature IDs, Perfil de análisis, Runtime experimental, Explicaciones y Contrato de negativos léxicos y sintácticos. |
| AST | [parse.rs](../../src/frontend/parse.rs), declaraciones `Ast`…`Argument`, `statement_inner`, `prefix`, `if_form`. |
| LEX | [lex.rs](../../src/frontend/lex.rs), `Kind`, `Token`, `Lexer::scan`, `is_reserved`, `longest_symbol`. |
| PRINT | [print.rs](../../src/frontend/print.rs), `print_canonical`, `ty`, `stmt`, `expr`, `print_else`. |
| MOD | [mod.rs](../../src/frontend/mod.rs), exportaciones `lex`, `parse`, `print`. |

La referencia de implementación es el commit vigente al aprobar este ADR; el inventario de AST corresponde a `src/frontend/parse.rs`. Se distingue **F0**, el perfil ejecutable de A2, de **B completa**, la superficie de diseño que incluye D11–D13 y EX01–EX07. Esta distinción es de capacidades, no de dialectos: una forma diferida tiene una sola grafía y un rechazo de alcance, nunca una traducción alternativa aceptada.

No cambia la semántica de copia, préstamo, transferencia, evaluación, efectos, limpieza, nominalidad, exclusividad, retorno o propagación. `move` nombra la transferencia; `edit`, el permiso exclusivo; `view`, la lectura de un parámetro/captura. Se compone mediante llamadas y selección con punto. No hay tubería `|>`, `fails`, `prop`, `defer`, `take` como modo, operador general `copy`, retorno implícito, `T?`, `?.`, `??`, `try` o `fail` como control de errores. `Fail()` permanece únicamente como intrínseca experimental de A2 §8.4. No se importan las listas dinámicas implícitas, tipos `Int`/`Unit`, métodos dentro de struct, campos `var`, lambdas con flecha, `module`, `private`, `iter` o `taskgroup` de EXT.

Las secciones 2–5 bastan para escribir el frontend sin consultar una gramática anterior. Las referencias sustentan hechos semánticos y la trazabilidad; no completan producciones omitidas de F0. Las formas de diseño diferidas también se escriben explícitamente en §3.5.

## 2. Léxico completo

### 2.1. Bytes, alfabeto y trivia

La entrada es UTF-8 válido. Un archivo ilegible o UTF-8 inválido sigue A2 §§11 y 13.6, antes del lexer. No se normalizan Unicode ni finales de línea antes de calcular spans. Los spans son intervalos semiabiertos de bytes originales, base cero. LF, CR y CRLF son cada uno un salto lógico; CRLF ocupa dos bytes. EOF tiene texto vacío y span `[bytes,bytes)`.

Identificador: `[A-Za-z_][A-Za-z0-9_]*`, coincidencia máxima, sensible a caja. `_` es un nombre en F0 y comodín solo en posición de patrón en B completa. Unicode se admite en comentarios y cadenas, no en identificadores. El espacio horizontal es la lista cerrada U+0009, U+000B, U+000C, U+0020, U+0085, U+00A0, U+1680, U+2000…U+200A, U+2028, U+2029, U+202F, U+205F y U+3000; estos caracteres separan tokens, pero no insertan terminadores. Esto hace explícito el comportamiento `char::is_whitespace` de LEX sin depender de una futura versión de Unicode. Solo CR/LF son saltos de la gramática. U+FEFF no es trivia: el BOM de fuente se rechaza como NX-E0001. El presente ADR y la salida canónica son UTF-8 sin BOM, LF.

`//` inicia comentario hasta CR, LF o EOF, sin consumir el salto. `///` usa el mismo léxico y añade asociación documental en el CST. No existen comentarios de bloque. Las cadenas y comentarios preservan sus bytes; el CST conserva trivia, agrupaciones y el mapa de origen, aunque el AST semántico no los almacene (LEX/AST; separación requerida por D14 §10.1).

### 2.2. Palabras reservadas y contextuales

| Clase | Lista cerrada |
|---|---|
| Reservadas F0 | `fn let var return if then else while true false with` |
| Reservadas de B completa, reconocidas pero no aceptadas en F0 | `and or not match for break continue gen yield async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package` |
| Contextuales, siempre token Name | `view edit move copy value from result_from retains as in create call resume cancel drop unrestricted no_alloc no_heap_alloc no_block no_suspend no_io capacity executor local migrable layout derive` |

Los restantes identificadores son Name, incluidos nombres de tipos, preámbulo, `take`, `borrow`, `fails`, `prop` y `defer`. Que un nombre exista léxicamente no crea una construcción: `take` donde se requiere modo produce NX-E0002; `let take = 1;` es un binding ordinario. `inout` y `consume` son Name ordinarios, sin función de modo o prefijo: `let consume=1;` es válido, mientras `x:inout I64` falla como cualquier modo ajeno a ParamMode. No se reserva vocabulario de A para conservar rastros de su gramática. No existe `legacy_move` en B.

Reglas de selección contextual, antes de consultar tipos o símbolos:

1. Al comienzo de sentencia, Name `value` seguido de `:` **en la misma línea lógica** selecciona entrega de rama. En cualquier otra posición es nombre. `let value: I64 = 1;`, `F(value: 1)`, `value -1`, `value(x)` y `value[0]` no son entregas. El compilador reconoce la entrega mal situada para su diagnóstico específico.
2. Después de `nombre:` en un parámetro, `view/edit/move` es modo obligatorio. En una captura, el primer Name debe ser `view/copy/edit/move` y el siguiente es el binding. `view` fuera de esas posiciones no es un operador ni un tipo prestado.
3. En expresión, `edit/move` seguido por un comienzo de Postfix se elige prioritariamente como prefijo; sin ese comienzo es Name. La anticipación ignora trivia y saltos. `F(move)` lee un binding; `F(move x)` transfiere; `move(x)` transfiere el lugar agrupado; `(move)(x)` llama un binding. `move - 1` selecciona prefijo sobre entero firmado y falla por Place; para restar de un binding homónimo se escribe `(move) - 1`. `F(move)` sin declaración es un error de nombre, no de modo faltante. En `fn [move]`, que ya exige modo, falta el nombre: NX-E0002 sobre `]`, esperado `["Name"]`.
4. Una etiqueta `Name :` en Argument se reconoce antes de analizar su expresión: `F(move: x)` es argumento etiquetado, no transferencia. `edit view` permite usar el binding llamado `view` de EX03.

Estas reglas concretan D14 §§5.1–5.2 y 7.4. `copy` únicamente indica copia de captura o una operación de contrato. `drop` en un contrato y `Drop` como nombre resuelto son categorías diferentes, sin inferencia por caja.

### 2.3. Convención de nombres y preámbulo

| Categoría resuelta | Forma exigida por el formateador |
|---|---|
| Tipos, traits, variantes y parámetros de tipo | PascalCase |
| Funciones y métodos declarados, privados o públicos | PascalCase |
| Campos y constantes públicos | PascalCase |
| Variables, parámetros, bindings de patrón, closures almacenadas | camelCase |
| Campos y constantes privados | camelCase |
| Segmentos de módulo/paquete | Minúsculas; snake_case si tienen varias palabras |
| Contratos, modos y palabras del lenguaje | Literales de §2.2 |

Una etiqueta reproduce la declaración de su parámetro/campo. `Main` es la identidad de entrada de `run`; una función ordinaria `main` sigue siendo un nombre legal, pero no es la entrada ni un alias de `Main`. El compilador no clasifica funciones/tipos por su mayúscula. Las grafías intrínsecas son entradas explícitas de un manifiesto, como cualquier API sensible a caja (D14 §§2.2–2.3; A2 §§4.4 y 8.4).

Para convertir nombres históricos: dividir por `_`, por transición minúscula→mayúscula y antes de la última mayúscula de una secuencia seguida por minúscula; convertir palabras a minúscula y capitalizar todas para PascalCase o todas salvo la primera para camelCase. Dígitos permanecen en su palabra. No añadir sufijos ante colisión. Excepciones por identidad de biblioteca: layout `SoA`→`Soa` y `AoS`→`Aos`; no afectan homónimos de usuario. Ejemplos: `HTTPServer`→`HttpServer`, `read_text`→`ReadText`, `max_connections`→`maxConnections`. El estilo se comprueba con información de declaración, no con heurísticas del lexer.

Mapa completo de primitivas/operaciones de **A2 §8.4**, conservando identidad, firma y efectos. Las etiquetas aquí publicadas fijan los nombres de parámetros de las intrínsecas; las firmas de usuario pueden sombrear sus nombres, sin heredar privilegios:

| Identidad histórica estable | Nombre B / firma de superficie | Efectos del modelo |
|---|---|---|
| `mock_open` | `MockOpen(label: view I64) -> MockResource` | `io` |
| `MockResource.drop` | Destructor de `MockResource`; grafía descriptiva `MockResource.Drop`, sin llamada de método aceptada en F0 | `io` |
| `mock_blocking_open` | `MockBlockingOpen(label: view I64) -> MockBlockingResource` | `io` |
| `MockBlockingResource.drop` | Destructor de `MockBlockingResource`; grafía descriptiva `MockBlockingResource.Drop` | `block, io` |
| `inspect` | `Inspect(r: view MockResource) -> ()`, también el otro recurso por dominio intrínseco | `io` |
| `drop` | `Drop(v)`; destrucción propietaria intrínseca, no una firma ordinaria `view` | Destrucción del tipo; en ejecución solo payload activo |
| `len` | `Len(a: view Array<T>) -> I64` | Vacío |
| `try_add` | `TryAdd(a: view I64, b: view I64) -> Result<I64, I64>` | Vacío |
| `assert_eq` | `AssertEq(a: view T, b: view T) -> ()`; mismo tipo de datos copiables | Vacío; posible R0201 |
| `print` | `Print(v: view T) -> ()`; datos copiables | `io` |
| `fail` | `Fail() -> ()` | Fallo experimental y limpieza de salida |
| `mock_read_records` | `MockReadRecords(r: view MockResource) -> Result<Array<Result<I64, I64>>, I64>` | `alloc, io` |
| `mock_write_i64` | `MockWriteI64(n: view I64) -> Result<(), I64>` | `io` |
| `mock_request` | `MockRequest() -> Result<I64, I64>` | `io` |
| `mock_reply` | `MockReply(status: view I64) -> Result<(), I64>` | `io` |
| `mock_unknown` | `MockUnknown() -> ()` | Desconocidos los cuatro efectos |

`T` en esa tabla expresa el dominio del intrínseco, no genéricos de usuario habilitados. `Drop` consume por identidad resuelta incluso sin `move`; una función de usuario `Drop` sigue su firma ordinaria. No se renombran los eventos `open`, `inspect`, `close`, las claves JSON ni `effect_model = f0-0002-sim-v1` (A2 §§6.5, 8.4–8.5, 12.4).

| Otras identidades de API | Grafías B |
|---|---|
| Entrada; tipos F0; unidad | `main`→`Main`; `i64`→`I64`; `bool`→`Bool`; `Array`, `Result`, `MockResource`, `MockBlockingResource`, `()` sin cambio |
| Constructores Result | `Ok`, `Err` sin cambio; etiqueta de payload `v` para ambos, conservando `src/check/names.rs::intrinsic` |
| Tipos fuera de F0 de los ejemplos | `u32/u64/f32/f64`→`U32/U64/F32/F64`; `String`, `Option`, `Future`, `TaskHandle`, `TaskOutcome`, `Call`, `CallMut`, `CallOnce` sin cambio |
| Constructores diferidos | `Some`, `None`, `Completed`, `Cancelled` sin cambio |
| EX01 | `cli.Positional`, `fs.ReadText`, `Lines`, `Trim`, `IsEmpty`, `parse.I64`, `TryAdd`, `console.Println` |
| EX02 | `json.Encode`, `http.Bind`, `http.Serve`, `Response.Text`, `Response.Json`, `RequestView.Method`, `RequestView.Path`, `Method.Get` |
| Etiquetas EX02 | `maxConnections`, `maxHeaderBytes`, `maxBodyBytes`, `idleTimeoutMs`; `handler`, `estado` sin cambio |
| EX03 | `Table.FromValues`, `PrepareUnique`, `Row`, `Soa`, `Aos`; etiquetas `layout`, `capacity`, `dt` sin cambio |
| EX04/D10; EX06/D12 | `Next`, `Spawn`, `ByteLen`; `Drop` según la tabla anterior |

Las rutas `std.app`, `std.cli`, `std.console`, `std.fs`, `std.parse`, `std.http`, `std.json`, `std.table`, `app.Error` permanecen. El resto de nombres declarados de los ejemplos aparece literalmente en §8. No se inventan APIs de cancelación o ejecución async. Las operaciones descriptivas `create/copy/call/resume/cancel/drop` de los contratos no se convierten a PascalCase.

### 2.4. Literales y símbolos

Entero tokenizable: uno o más dígitos ASCII, sin separador, base, sufijo ni exponente. El signo es otro token. En F0, el parser une `-` y un token entero cuando espera un operando, incluso con trivia: `-9223372036854775808` es un literal válido de I64; `-x` no lo es. Tras un operando `-` es resta; `value -1` y `value - 1` son equivalentes. Se conserva el texto de dígitos (incluidos ceros iniciales) en `Expr::Integer`; no se transforma por formato. El rango numérico se comprueba sobre el valor firmado, con NX-E0107 (A2 §§3.5 y 4; AST `prefix`).

Booleanos: `true`, `false`. Unidad: `()`; no hay alias `Unit`. Decimal reconocido: `Digits "." Digits`, sin absorber un punto seguido por otro punto. `0.5` es decimal; `0..<n` son entero, rango y nombre. Decimales son `float_literal`, no aceptados en F0. B completa conserva esos literales; no fija nuevos redondeos ni exponentes.

Cadena de **B completa**: comillas dobles, contenido Unicode sin CR/LF crudos, escapes cerrados `\"`, `\\`, `\n`, `\r`, `\t`; `{{` y `}}` representan llaves literales. `{Expr}` interpola una expresión, una vez y en orden escrito; no admite sentencias ni campos de formato. Las comillas y escapes de interpolaciones se analizan recursivamente en su contexto. No hay raw strings, triples ni plantillas UI (D14 §3.2; R D10).

En **F0**, el lexer reconoce una envoltura simple: termina en la primera `"` no escapada, sin analizar interpolaciones ni habilitar comillas interiores. Valida los cinco escapes anteriores; un escape desconocido es NX-E0001 sobre ese escape, con el contrato léxico de §6.3. Las llaves simples o duplicadas son contenido opaco: no se exige duplicarlas ni se comprueba el balance de una interpolación. Una cadena incompleta ocupa desde `"` hasta EOF o antes del CR/LF que la interrumpe; una barra inversa no permite atravesar ese salto. Una vez obtenido el token, el parser emite NX-E0006/feature_id `text` sobre el token completo **sin mirar su interior**, conforme al reconocimiento de alcance de A2 §§3.2–3.6. Así `"…{x}"` se rechaza por `text`, sin resolver x ni ejecutar interpolación. La recursión y el límite de anidamiento de interpolaciones se incorporan al habilitar esa capacidad.

Regresiones literales sin LF final, con offsets UTF-8; main→Main no cambia los offsets. `src/frontend/lex.rs::Lexer::scan` ya admite llaves simples, pero acepta cualquier carácter tras `\`; la validación de los cinco escapes es un cambio exigido por B, no comportamiento actual acreditado:

| Fuente B | Comportamiento actual con main | Esperado B |
|---|---|---|
| `fn Main(){let s="…{x}";}` | NX-E0006/text, primario `[16,24)` | El mismo código/feature_id y primario; no NX-E0001 sobre `{` en `[20,21)`. |
| `fn Main(){let s="{{x}}";}` | NX-E0006/text, primario `[16,23)` | El mismo código/feature_id y primario. |
| `fn Main(){let s="\q";}` | NX-E0006/text, primario `[16,20)` | NX-E0001, token invalid `\q` y primario `[17,19)`, recognized_prefix `[0,17)`; restantes campos léxicos de §6.3. |

En las filas text: subcause null, parse/syntax.parse, data `{"pending_scope":"later_adr"}`, token/parser_context/recognized_prefix/expected_tokens_kind null y expected_tokens/related_spans/cause_chain/repair vacíos; perfil B de §6.1. El positivo de la fila léxica cambia únicamente `\q` por `\n`: supera léxico y conserva NX-E0006/text `[16,20)`; para superar también la exclusión text se sustituye el literal completo por `0`.

Inventario **cerrado** de símbolos B, probados por coincidencia más larga:

```text
..< ..= += != -> => == <= >=
( ) { } [ ] , : ; ? + - < > = @ . * / %
```

El lexer reconoce adicionalmente `..` como token retirado para error de extremo explícito; nunca como rango aceptado. Se prueba `..<` y `..=` antes de `..` y `.`. `//` precede a `/`. `>` se tokeniza individualmente, sin `>>`; `>=` se puede dividir en `>` y `=` exclusivamente al cerrar argumentos de tipo, conservando spans de un byte. `Result<I64,I64>=Ok(1)` no es una comparación. `/`, `%` son operadores de B completa pero no aceptados en F0; `!`, `&`, `|`, `^`, `~` y `\` fuera de cadenas/comentarios son NX-E0001. `&&`, `||`, `|>` no forman tokens compuestos aceptados. `!` dentro de `!=` sí pertenece al token de desigualdad.

Los símbolos hacen trabajos diferenciados por producción: `:` anota tipo/modo, etiqueta o entrega; `with` introduce contrato en firma o guarda en sentencia; `copy` captura o etiqueta de contrato; `< >` comparan o delimitan tipos; `and` conjuga booleanos o cotas; `[]` delimita arrays, índices o capturas; `{}` cuerpos, imports, contratos e interpolaciones; `()` agrupa, llama, delimita parámetros/patrones/contratos o unidad; `=` inicializa/asigna/define alias; `-` signo/resta; `.` selecciona/califica; `=>` solo abre brazo de match. Las sobrecargas de `< >`, `and`, `with`, `:` y `copy` son deuda de D14 §6.2 y se mantienen. Frente al criterio 3 de AC §9.6, se justifica su conservación por contextos delimitados y producciones identificables, sin inferir tipos para elegir significado en F0; no se afirma una correspondencia universal de un símbolo con un solo concepto. La medición de comprensión de §9.3 observa estas cinco familias por separado; no introduce sinónimos ni reabre la superficie elegida.

### 2.5. Inserción de terminadores

`Terminator` es `;` escrito o un terminador insertado de ancho cero. La inserción no es un pase global de tokens: pertenece a la sentencia/declaración activa. Una expresión completa seguida por salto termina si no queda un paréntesis/corchete abierto **de esa sentencia** y el siguiente token no continúa su producción. Los delimitadores de una sentencia exterior no impiden terminar sentencias de un bloque de rama interior. Comentarios no borran el salto.

1. Continúan una expresión: `(`, `[`, `.`, `?`, `+`, `-`, `*`, `/`, `%`, `<`, `<=`, `>`, `>=`, `==`, `!=`, `and`, `or`, `..<`, `..=`, y una coma dentro de lista abierta. `..` retirado conserva la continuidad para denunciar el rango, sin fabricar NX-E0005. Los operadores aún excluidos de F0 conservan continuidad para su diagnóstico específico.
2. `then`, `else` y la apertura de cuerpo requerida mantienen una construcción de control aunque haya salto. Una función conserva su firma hasta su bloque; una declaración simple continúa ante los tokens que su producción admite, incluidos `.`, `as`, `=`, `->`, `from`, `result_from`, `retains`, `with`, `where`, `and` o coma en sus respectivas posiciones. No se inserta dentro de genéricos, grupos de import ni mapas `with` abiertos.
3. **Excepción prioritaria:** un salto inmediatamente después de `return` o del introductor `value:` termina esa sentencia vacía, incluso ante `(`. Para continuar se escribe `return (` o `value: (` antes del salto. Un salto después de un prefijo `move/edit` ya seleccionado no termina una expresión incompleta.
4. Un salto antes de `}` puede terminar una expresión completa; una `}` en la misma línea **nunca** inserta su terminador. EOF no crea un salto inexistente. Los cuerpos y sentencias estructurales terminan en su llave; una expresión que contiene un bloque todavía necesita el terminador de su sentencia exterior.
5. El terminador insertado se ancla en el final del último token de la sentencia, antes de trivia. No cuenta como byte leído, token inesperado, nodo semántico ni trabajo de ejecución. El CST registra si fue escrito o insertado y el salto que lo permite. Spans semánticos de sentencia excluyen el terminador insertado; incluyen `;` si está escrito (AST `terminator`; A2 §§3.1 y 13.15).
6. Si la producción está completa pero falta terminador, se reconoce `MissingTerminator` únicamente como recuperación errónea: NX-E0005 sobre el siguiente token/EOF, nunca aceptación. Una expresión incompleta no se «repara» insertándole `;`. `return;` puede ser válido para unidad; `value:;` siempre requiere el diagnóstico específico de operando ausente.

Advertencia: una línea sin `;` seguida de otra que comienza por `(`, `[` o `-` continúa la expresión anterior; el salto no las separa. Para iniciar una sentencia independiente se termina explícitamente la anterior con `;`. Una selección descartada se escribe `(if condición then a else b);` (§3.2), con el terminador previo explícito cuando comienza en una nueva línea tras otra expresión.

Dos sentencias simples en una línea requieren `;`. Dos declaraciones con cuerpo se separan por sus cierres. Los campos, variantes y requisitos de trait tienen sus reglas de §3.5; no confieren retorno implícito a un bloque (D13 §§1–2.1).

## 3. Gramática independiente de B

### 3.1. Notación, perfil F0 y entradas

EBNF: `|` alternativa, `{…}` repetición, `[…]` opcional, terminales entre comillas. `Name`, `Digits`, EOF y los símbolos provienen de §2. Los saltos/trivia se consumen por §2.5, no como operandos. Las restricciones contextuales que siguen son parte de la gramática normativa. No hay coma final salvo las excepciones explícitas de campos, variantes y brazos.

El parser F0 requiere lookahead de dos tokens significativos, LL(2), para `Name ":"` en etiquetas, `value:` al inicio de sentencia y la selección de prefijo `edit/move`; aplica además las reglas de salto de §2.5. No basta LL(1). En B completa, `GenericHead` requiere análisis especulativo de la lista de tipos con restauración al fallar y la condición de adyacencia sensible a espacios de §3.5; no se promete LL(2) para esa extensión.

Esta es la gramática **aceptada sintácticamente en F0**, antes de tipos, nombres, modos y flujo. Los reconocedores inválidos de §3.4 y las exclusiones de §3.6 se aplican con su prioridad específica.

```ebnf
Program       = {Function} EOF ;
Function      = "fn" Name "(" [Parameters] ")"
                ["->" Type] [WithClause] Block ;
Parameters    = Parameter {"," Parameter} ;
Parameter     = Name ":" ParamMode Type ;
ParamMode     = "view" | "edit" | "move" ;
Type          = "I64" | "Bool" | "(" ")"
              | "MockResource" | "MockBlockingResource"
              | "Array" "<" Type ">"
              | "Result" "<" Type "," Type ">" ;

Block         = "{" {Statement} "}" ;
Statement     = Binding | Assignment | ReturnStmt | ValueStmt
              | IfStmt | WhileStmt | Block | ExprStmt ;
Binding       = ("let" | "var") Name [":" Type] "=" Expr Terminator ;
Assignment    = Place "=" Expr Terminator ;
ReturnStmt    = "return" [Expr] Terminator ;
ValueStmt     = ValueName ":" [Expr] Terminator ;
ValueName     = Name ;
IfStmt        = "if" Expr Block ["else" (Block | IfStmt)] ;
WhileStmt     = "while" Expr Block ;
ExprStmt      = CompareExpr Terminator ;

Expr          = IfExpr | CompareExpr ;
IfExpr        = "if" Expr "then" BranchOperand "else" BranchOperand ;
BranchOperand = CompareExpr | BranchBlock ;
BranchBlock   = "{" {Statement} "}" ;
CompareExpr   = AddExpr [("<" | "==") AddExpr] ;
AddExpr       = TryExpr {("+" | "-") TryExpr} ;
TryExpr       = AccessExpr ["?"] ;
AccessExpr    = [AccessMode] Postfix ;
AccessMode    = "edit" | "move" ;
Postfix       = Primary {Index | Arguments} ;
Index         = "[" Expr "]" ;
Arguments     = "(" [Argument {"," Argument}] ")" ;
Argument      = [Name ":"] Expr ;
Primary       = IntLiteral | "true" | "false" | Name
              | "(" [Expr] ")" | "[" [Expr {"," Expr}] "]" ;
IntLiteral    = ["-"] Digits ;
Place         = PlaceRoot {Index} ;
PlaceRoot     = Name | "(" Place ")" ;
Terminator    = ";" | InsertedTerminator ;

WithClause     = "with" (Restrictions | OperationList) ;
OperationList  = "{" [OperationLimit {"," OperationLimit}] "}" ;
OperationLimit = Operation "(" Restrictions ")" ;
Operation      = "create" | "copy" | "call" | "resume" | "cancel" | "drop" ;
Restrictions   = RestrictionItem {"," RestrictionItem} ;
RestrictionItem = "unrestricted" | Restriction ;
Restriction    = "no_alloc" | "no_heap_alloc" | "no_block" | "no_suspend" | "no_io" ;
```

`ValueName` requiere el selector exacto de §2.2. `InsertedTerminator` requiere §2.5. El programa vacío es biblioteca válida; `run` exige `Main` sin parámetros y resultado `()` o `Result<(), I64>`. Omitir `->` significa unidad en F0; inferir otro resultado sigue diferido con `inferred_non_unit_result` (A2 §§4.1 y 4.4).

Después de reconocer un prefijo de expresión que es Place, `=` selecciona Assignment; sin `=`, se conserva ExprStmt. `IfStmt` no es una expresión descartada. Para descartar una selección se escribe `(if c then a else b);`. `if` expresión usado como operando de otra operación requiere agrupación. Una condición puede ser otra selección completa, delimitada por su propio `then/else`; la recursión de Expr decide esos límites sin tipos. Una rama que contenga otra selección exige paréntesis: `if a then x else (if b then y else z)`. No se acepta `else if` sin paréntesis **en posición expresión**; `else if` sentencia sigue IfStmt.

### 3.2. Precedencia, modos y equivalencia de selección

F0: postfix (llamada/índice), modo, `?`, suma/resta, comparación, selección, de fuerte a débil. Suma/resta asocian a izquierda; comparaciones no encadenan. `?` aparece una sola vez por TryExpr; `(F()?)?` es una nueva expresión agrupada, mientras `F()??`, `F()?(x)` o `F()?[0]` no derivan esta gramática. Para indexar el resultado propagado: `(F()?)[0]`. `move r?` significa `(move r)?`, nunca mover el payload implícitamente.

Se mantiene el dominio F0 de A2 §§3.5–3.6 y 4.6: solo `<` y `==` se aceptan. `*`, `/`, `%`, `!=`, `>`, `<=`, `>=`, `and` y `or` se tokenizan pero producen NX-E0002 en el reconocedor de operador excluido; `not` fuera de una producción F0 produce NX-E0002. B completa fija su precedencia en §3.5. La migración de superficie no implementa aritmética ni lógica adicional. `and/or` en posición infija usan `parser_context=unsupported_binary_operator`, `expected_tokens=["+","-","<","=="]`; `not` al comienzo de un operando usa `parser_context=unsupported_unary_operator`, `expected_tokens=["(","-","IntLiteral","Name","[","edit","false","if","move","true"]`. Los tres tienen NX-E0002, subcause null, stage parse, operation syntax.parse, token kind keyword sobre la palabra completa, expected_tokens_kind admissible_set, data {}, repair [] y recognized_prefix desde el comienzo de la sentencia activa hasta ese token. En posiciones que exigen un nombre rige el contexto de nombre de §6.2. Estas causas preceden al terminador ausente derivado. Una selección descartada en posición de sentencia usa `(if condición then a else b);`; sin paréntesis, el introductor `if` selecciona IfStmt.

Una llamada conserva el orden escrito de argumentos, antes de asociar etiquetas a parámetros; tras el primero nombrado, todos los siguientes deben ser nombrados. No duplicar, omitir ni desconocer etiquetas; los incumplimientos conservan NX-E0111 de A2 §§4.2–4.3. Un argumento `x: move r` es una etiqueta y una expresión de transferencia; no mueve por el mero modo de la firma. Los temporales propietarios nuevos pueden entregarse sin prefijo.

Clasificación de prefijos F0, en este orden (A2 §3.6): llamada completa → `receiver_mode`; operando que no es Place → NX-E0002, contexto estable `consume_place` o `inout_place`; `move` de Place con índices → `partial_transfer`; binding completo o Place editable → comprobación semántica. `move (x)` conserva transferencia completa. `edit` no se acepta como permiso general fuera de los contextos admitidos. `view` no es un prefijo de expresión.

Cada rama simple de IfExpr se expande **una vez** a `Block([Value(Some(operando))])`, con origen sobre el operando escrito; no se inserta un Return. Hay una excepción estructural precisa para conservar las cadenas actuales: si el operando simple de **else**, después de quitar sus Group exteriores, es IfExpr, se representa directamente como `Else::If`, sin Block/Value envolventes. En `then` no se aplica esa excepción. Cada bloque explícito conserva sus sentencias, incluido `else { value: (if … then … else …); }`, que sigue siendo `Else::Block`; el impresor no lo abrevia a la forma de cadena. Así se preservan tanto los dos árboles actuales como sus cargos de trabajo, sin cambiar el intérprete. Todo camino normal de una rama debe terminar con entrega directa; una salida Return o divergencia no requiere entrega. No se sintetiza valor de la última expresión. La entrega limpia locales de rama antes de continuar y no abandona el invocable (D13 §§1, 3, 5–7; D14 §5.2).

Esta es una precisión expresa de la expansión y abreviación de D14 §5.2 al confrontarlas con `Else::If` del AST real: esa sección no distinguía la cadena directa del bloque que entrega otra selección. Se conserva su gramática de operandos y se restringe únicamente esa abreviación; fundir ambos árboles violaría la correspondencia requerida. La migración debe incluir positivos separados de ambas formas y comprobar también sus costes, sin usar una normalización que oculte la diferencia.

### 3.3. Contratos reconocidos y aceptación

WithClause tiene exactamente la EBNF de §3.1 en todas las posiciones. Una lista de restricciones no está vacía, no repite restricciones y solo admite `unrestricted` como único elemento. Un mapa no está vacío, no repite operaciones y usa únicamente las operaciones pertinentes. `no_heap_alloc` sigue reconocido pero no aceptado en F0. Estas condiciones se comprueban después de reconocer las formas necesarias para los errores específicos, sin aceptar mapas mixtos como `with no_io, create(no_alloc)`.

| Contexto | Lista simple | Operaciones del mapa y obligaciones |
|---|---|---|
| Función ordinaria | Limita `call` | Solo `call`, obligatoria si hay mapa |
| Closure síncrona | Limita `call` | `create copy call drop`; `call` obligatoria |
| Async privada/local, incluida closure | Limita `resume` del futuro | `create resume cancel drop`; `resume` obligatoria |
| Async pública | Prohibida | Las cuatro `create resume cancel drop` |
| Cota Call/CallMut/CallOnce | Prohibida | `create copy call drop`; `call` privada, las cuatro pública |
| Future abstracto escrito | Prohibida | Las cuatro `create resume cancel drop` |

Ausencia total de contrato en cuerpo privado permite inferencia local; en cota invocable da `missing_with_clause`. Prioridad: cláusula ausente; mapa vacío; primera duplicación; mezcla `unrestricted`; operación inadmisible antes de ausencia derivada de operación; omisión de interfaz con código específico antes de `missing_body_operation`. Se preservan NX-E0417 y los específicos NX-E1112/NX-E1114/NX-E1202 (D11 §5.2; D12 §4). Se conserva el orden de elementos del AST; el impresor no ordena ni elimina restricciones o capturas.

### 3.4. Formas inválidas reconocidas, sin compatibilidad

| Forma | Tratamiento obligatorio |
|---|---|
| `if c then rama` sin `else` | Recuperar `Expr::If.otherwise = None` únicamente para emitir NX-E1302/missing_else sobre `if`; nunca entregar un programa aceptado. |
| `if c {…} else {…}` en expresión | NX-E0002 sobre `{`, esperado `["then"]`; no analizarla como expresión A. |
| `value:` vacío o fuera de rama | Reconocer para NX-E1302; prioridad `invalid_value_target`, `missing_value_operand`, `non_final_branch_value`, errores de flujo derivados. |
| Rama `{ E; }` con continuación normal | NX-E1302/missing_branch_value; no convertir E en retorno o entrega. |
| Función no unitaria que cae al final | NX-E1317, sin inventar una última expresión implícita. |
| `..` como operador | NX-E0002 sobre `..`, contexto `range_endpoint`, esperados `["..<","..="]`; explicación pide elegir extremo; no edición automática que adivine inclusividad. |
| `=> return E` o `=> E` en cuerpo de función/closure B completa | NX-E0002 sobre `=>`, esperado `["{"]`, contexto `grammar.ClosureBody` o `grammar.FnDecl`. No se conserva la abreviatura D11 ni su reparación a otra flecha. En F0 el introductor de closure llega primero a `closure`. |
| Coma final en parámetros, argumentos, tipos, arrays, imports, capturas o contratos | NX-E0002 sobre el cierre, FIRST del elemento obligatorio siguiente. |
| `async gen fn` en B completa | Reconocido para NX-E1206/async_generator_declaration; no se implementan generadores async. |

La opcionalidad de `else` para **recuperación** no es una segunda sintaxis válida. El AST tiene espacios representables para programas inválidos; no todos los valores que puede construir Rust pertenecen al dominio del impresor de programas aceptados.

### 3.5. B completa: formas diferidas, con grafía fijada

Todas las ampliaciones de esta sección están **reconocidas pero no aceptadas en F0** cuando alcancen una exclusión de §3.6. No implican nuevos nodos ejecutables en el corte de migración. Aquí `Type`, `Expr`, `Statement` y `Place` designan las versiones completas siguientes; las producciones comunes de §3.1 se instancian con ellas. `Parameter` F0 se sustituye por ParamDecl en firmas completas. `WithClause` conserva su única definición.

```ebnf
Module        = {ModuleDecl} EOF ;
ModuleDecl    = {BaseAttribute} BareModuleDecl ;
BaseAttribute = "@" "derive" "(" Path {"," Path} ")" ;
BareModuleDecl = ConstDecl | AliasDecl | FnDecl | StructDecl | EnumDecl
               | TraitDecl | ImplDecl | ExtendDecl | ModDecl | UseDecl ;
ConstDecl     = ["pub"] "const" Name ":" Type "=" "comptime" Expr Terminator ;
AliasDecl     = ["pub"] "type" Name "=" Type Terminator ;
FnDecl        = ["pub"] ["async"] ["gen"] "fn" Name [GenericParams]
                Signature [WhereClause] Block ;
Signature     = "(" [ParamDecl {"," ParamDecl}] ")" ["->" ResultType]
                ProvenanceClauses [WithClause] ;
ParamDecl     = (Name ":" ParamMode Type | "self" ":" ParamMode) ["=" Expr] ;
GenericParams = "<" GenericParam {"," GenericParam} ">" ;
GenericParam  = Name [":" Bound {"and" Bound}] ;
Bound         = CallableBound | TypePath ;
WhereClause   = "where" Constraint {"," Constraint} ;
Constraint    = Name ":" Bound {"and" Bound} ;
StructDecl    = ["pub"] "struct" Name [GenericParams] "{"
                [Field {FieldSeparator Field} [FieldSeparator]] "}" ;
Field         = ["pub"] Name ":" Type ;
FieldSeparator = "," | ";" | InsertedTerminator ;
EnumDecl      = ["pub"] "enum" Name [GenericParams] "{"
                [Variant {"," Variant} [","]] "}" ;
Variant       = Name ["(" Type {"," Type} ")"] ;
TraitDecl     = ["pub"] "trait" Name [GenericParams] "{" {TraitMember} "}" ;
TraitMember   = "fn" Name [GenericParams] Signature [WhereClause] (Terminator | Block) ;
ImplDecl      = "impl" [GenericParams] TypePath "for" Type [WhereClause]
                "{" {MethodDecl} "}" ;
ExtendDecl    = "extend" Type "{" {MethodDecl} "}" ;
MethodDecl    = ["pub"] "fn" Name [GenericParams] Signature [WhereClause] Block ;
ModDecl       = ["pub"] "mod" Name Terminator ;
UseDecl       = ["pub"] "use" (ImportSpec | "extension" Path) Terminator ;
ImportSpec    = Path ["as" Name] | Path "." "{" ImportName {"," ImportName} "}" ;
ImportName    = Name ["as" Name] ;
Path          = PathHead {"." Name} ;
PathHead      = Name | "self" | "super" | "package" ;
Type          = TypePath | "(" ")" | "(" FutureType ")" | "dyn" "(" CallableBound ")" ;
TypePath      = Path [TypeArgs] ;
TypeArgs      = "<" TypeArg {"," TypeArg} ">" ;
TypeArg       = Type | Name ":" ConstTypeArg ;
ConstTypeArg  = Path | IntLiteral | "true" | "false" ;
ResultType    = Type ;
CallableBound = ("Call" | "CallMut" | "CallOnce")
                "(" [CallableParam {"," CallableParam}] ")" "->" ResultType
                ProvenanceClauses [WithClause] ;
CallableParam = [Name ":"] ParamMode Type ;
FutureType    = "Future" "<" Type ">" WithClause ;
ProvenanceClauses = [FromClause] [ResultFromClause] {RetainsClause} ;
FromClause    = "from" "(" Origin {"," Origin} ")" ;
ResultFromClause = "result_from" "(" Origin {"," Origin} ")" ;
RetainsClause = "retains" "(" Origin "," "in" ":" Origin ")" ;
Origin        = Name | "self" ;

Expr          = IfExpr | MatchExpr | OrExpr ;
IfExpr        = "if" Expr "then" BranchOperand "else" BranchOperand ;
BranchOperand = OrExpr | BranchBlock ;
OrExpr        = AndExpr {"or" AndExpr} ;
AndExpr       = CompareExpr {"and" CompareExpr} ;
CompareExpr   = RangeExpr [("<" | "<=" | ">" | ">=" | "==" | "!=") RangeExpr] ;
RangeExpr     = AddExpr [("..<" | "..=") AddExpr] ;
AddExpr       = MulExpr {("+" | "-") MulExpr} ;
MulExpr       = NegExpr {("*" | "/" | "%") NegExpr} ;
NegExpr       = ("not" | "-") NegExpr | TryExpr ;
TryExpr       = (AwaitExpr | AccessExpr) ["?"] ;
AwaitExpr     = "await" AccessExpr ;
AccessExpr    = [AccessMode] Postfix ;
Postfix       = Primary {Selector} ;
Selector      = "." Name | Index | Arguments ;
Primary       = Digits | Decimal | StringLiteral | "true" | "false" | Name
              | "self" | "super" | "package" | GenericHead
              | "(" [Expr] ")" | "[" [Expr {"," Expr}] "]"
              | ClosureExpr | "extension" Path ;
GenericHead   = Path TypeArgs ;
Place         = PlaceRoot {"." Name | Index} ;
PlaceRoot     = Name | "self" | "(" Place ")" ;
ClosureExpr   = ["async"] "fn" [CaptureList]
                "(" [ClosureParam {"," ClosureParam}] ")" ["->" ResultType]
                ProvenanceClauses [WithClause] ClosureBody ;
ClosureBody   = Block ;
ClosureParam  = Name ":" ParamMode [Type] ;
CaptureList   = "[" [Capture {"," Capture}] "]" ;
Capture       = ("view" | "copy" | "edit" | "move") Name ;
MatchExpr     = "match" Expr "{" [Arm {"," Arm} [","]] "}" ;
MatchStmt     = "match" Expr "{" [Arm {"," Arm} [","]] "}" ;
Arm           = Pattern ["if" Expr] "=>" Block ;
Pattern       = "_" | IntLiteral | "true" | "false" | AccessMode Name
              | Path ["(" Pattern {"," Pattern} ")"]
              | "(" Pattern {"," Pattern} ")" ;
Statement     = Binding | Assignment | ReturnStmt | ValueStmt | YieldStmt
              | ExprStmt | IfStmt | WhileStmt | ForStmt | LoopControl
              | MatchStmt | Block | RegionStmt | GuardStmt ;
Assignment    = Place ("=" | "+=") Expr Terminator ;
ExprStmt      = OrExpr Terminator ;
YieldStmt     = "yield" Expr Terminator ;
ForStmt       = "for" ["edit"] Name "in" Expr Block ;
LoopControl   = ("break" | "continue") Terminator ;
RegionStmt    = "region" Name "(" "capacity" ":" Expr
                ["," "executor" ":" ("local" | "migrable")] ")" Block ;
GuardStmt     = "with" Expr "as" Name Block ;
```

`Decimal` y `StringLiteral` son los literales de §2.4. IntLiteral conserva su definición para patrones. La negación completa tiene la precedencia indicada; el subconjunto F0 solo normaliza signo aplicado a entero. No se introducen tuplas de valores completas por la alternativa de patrones; la coma de una agrupación de expresión sigue un reconocedor `tuple` hasta un ADR de esa capacidad.

Reglas de desambiguación y semántica que cierran estas producciones:

- GenericHead exige adyacencia entre Path y `<`, lista TypeArgs bien formada y cierre seguido de `(` o `.`; se elige antes de comparación, sin consultar símbolos. El formato mantiene esa adyacencia y separa comparaciones con espacios. `F<I64>(x)` es genérico; `F < I64 > (x)` son comparaciones y no puede encadenarlas. Los cierres anidados son tokens `>` separados. `layout: Soa` es argumento constante nombrado del tipo, no un modo de parámetro. Las constantes admitidas aquí se limitan a paths y literales; el evaluador comptime y sus dominios conservan sus obligaciones de fase.
- `Future<T>` sin contrato escrito solo designa un futuro concreto ya determinado por contexto privado, como el parámetro de `Descartar` en EX06 (D14 §8.2, D12 §2). No es un futuro abstracto sin efectos conocidos. Un tipo de resultado indirecto o interfaz abstracta usa `(Future<T> with {…})`, con cuatro operaciones; se exige el contrato y no se toma TypePath como escapatoria de D11 §5.2. El parser reconoce ambas formas; resolución de contratos distingue concreto contextual de abstracto. Si falta esa determinación, no se presume vacío ni se borra identidad; se exige la forma con cotas.
- `and/or` cortocircuitan; `not` requiere paréntesis para negar una comparación completa. Rangos y comparaciones no encadenan; binarios aritméticos/lógicos asocian a izquierda. `..<` excluye el extremo; `..=` lo incluye sin calcular `hi + 1`; `lo > hi` recorre vacío, extremos del mismo tipo entero. Asignación y `+=` son sentencias y evalúan el lugar una sola vez. No hay operadores definidos por paquetes ni sobrecarga de control, préstamo o propagación (R D08; D14 §§3.2, 5.3).
- `await F()?` espera y luego propaga; `await (F()?)` propaga antes de esperar. `await move f` transfiere el futuro nombrado; una llamada async produce futuro inerte. `grupo.Spawn(F())` es llamada de API resuelta, no palabra `spawn`. Retornar de region solicita cancelación/reunión antes de destruir recursos. `gen fn` produce Item con `yield`, termina por caída o `return;`, nunca con `return E`, y conserva efecto de suspensión síncrona (D12 §§1–10; R D10; D13 §1.1).
- `x.M()` es forma de método; `(x.campo)()` llama al campo. El prefijo de `edit x.M()` o `move f()` califica el receptor de la última llamada, no su resultado ni llamadas anteriores. La calificación por módulo/tipo/trait se resuelve nominalmente; `Trait.M(edit x)` pasa receptor explícito. No se decide por mayúscula ni por encontrar un campo invocable (D13 G.1).
- Captura omitida presta por lectura todos los libres; lista presente exhaustiva, ordenada, sin duplicados y solo bindings completos. `[]` no captura; entradas no usadas conservan construcción/efectos/limpieza. `copy` no clona arbitrariamente; `move` invalida origen. Se preservan Call/CallMut/CallOnce, procedencias y los dos niveles de contrato de closure async/futuro (D11 §§2–5.3).
- Match expresión exige entrega final directa en cada continuación normal; match sentencia no es destinatario. No hay brazos `P => E`. Guardas preceden a transferencias y conservan restricciones de efectos; exhaustividad, tipos y coherencia son nominales. Las variantes nulas se resuelven por identidad, no por caja (D13 §§3–7).
- Módulo: raíz suministrada por manifiesto/CLI; `mod calculo` declara subarchivo, `use` importa, `pub use` reexporta solo lo exportable. Privado por defecto, `pub` explícito, sin inicializadores ambientales; alias y constantes según las producciones. Orden de import no da prioridad; extensiones se activan con `use extension`; no inventar `module ruta`, imports implícitos o conformidad estructural (D13 §§8–12; R D08).
- Firma pública exige tipos, procedencia y efectos suficientes; omisiones se reconocen para los diagnósticos específicos, sin inferencia pública accidental. `self: view/edit/move` toma tipo del trait/impl/extend; no existe nuevo tipo `Self`. Defaults son valores puros comptime y cada llamada obtiene valor lógico independiente. La omisión local de tipo de closure no permite omitir el modo (D14 §5.1; R D10).

### 3.6. Reconocedores F0 y fronteras cerradas

Para cadenas rige primero la validación léxica limitada de §2.4; el reconocedor de text no inspecciona el interior del token ya válido.

Los introductores diferidos, si están en la posición de su construcción, conservan NX-E0006, subcause null y la etapa pertinente de A2 §14.6. En posición de Name una reservada produce NX-E0002, no una exclusión de otra construcción. El lexer no anticipa NX-E0006 sobre un token posterior al introductor: se denuncia la primera exclusión del recorrido sintáctico. Un tipo reconocido debe tener delimitadores bien formados antes de denunciar alcance.

Lista cerrada de **30** feature_id F0: se conservan los 29 de alcance de B tras retirar `legacy_move` y se añade `implementation_limit` para la frontera temporal de §7.1:

```text
async attributes closure compound_assignment comptime default_argument
extensions float_literal for_iteration function_value generator
implementation_limit inferred_non_unit_result loop_control match methods modules_visibility
no_heap_alloc option partial_transfer provenance_clauses range receiver_mode
region resource_array text traits_generics tuple unsupported_type user_types
```

**Conjunto normativo: exactamente los 30 literales anteriores**; `indirect_call` permanece retirado por inalcanzable (A2 §3.4). El implementador debe comprobar la igualdad de conjuntos, no conservar cardinalidades documentales inconsistentes. `pending_scope` es `F0-0003` para `closure/generator/option/match`, `later_adr` para los restantes de alcance; `implementation_limit` usa exclusivamente `I08`, `I09`, `I10a`, `I10b` o `I11a` según §7.1. `migration` deja de emitirse. Esta entrada no retira ninguno de los 29 rechazos de alcance ni cambia sus reglas.

`implementation_limit` no existe en el código actual. Este ADR exige incorporarlo a `src/diagnostic.rs::FEATURE_IDS`, al catálogo explain y a la validación cruzada feature_id/pending_scope; el enum de valores admitidos de `pending_scope` añade los cinco literales I08/I09/I10a/I10b/I11a y retira migration. La inserción correspondiente en F0-CODIGOS se realiza junto con emisores y validadores del esquema, en «Feature IDs», «Datos estructurados» y «Explicaciones» según §6.5, dentro del mismo corte de implementación.

Tipos F0: las siete variantes de Type de §4. `Option`→`option`; `Future/TaskHandle/TaskOutcome`→`async`; una ruta no incluida→`unsupported_type` con la ruta completa y `data.type_spelling` original. Array con recurso transitivo→`resource_array` en tipos. `pub/mod/use`→`modules_visibility`; `fn` en posición de expresión→`closure`, con o sin lista; genéricos/trait/impl/where→`traits_generics`; métodos/selectores y `self` expresión→`methods`. La tabla literal F2-F02…F2-F33 de §8 fija las demás correspondencias. Los cinco operadores simbólicos excluidos originales conservan NX-E0002; los nuevos operadores completos excluidos `/ % and or not` también son sintaxis fuera de la gramática F0, sin crear nuevos feature_id.

## 4. Correspondencia exhaustiva con el AST actual

### 4.1. Tabla uno a uno de estructuras y variantes

Inventario tomado de las definiciones de AST, no de una gramática supuesta. Los nombres de campos Rust son internos y no se renombran por traducir la superficie. Todo span conserva origen B; ninguna expansión inventa bytes escritos.

| Nodo / variante actual | Producción B y ejemplo | Conservación |
|---|---|---|
| `Ast { functions, span }` | Program: `fn Main() {}` | Orden completo de funciones, hasta EOF |
| `Function` | Function: `fn F(x: view I64) -> I64 { return x; }` | name, name_span, params, result, with_clause, body, span |
| `Parameter` | Parameter: `x: edit I64` | Nombre, tipo, modo, span del parámetro completo |
| `Mode::Read` en parámetro | ParamMode `view` | Lectura, no copia ni nuevo tipo |
| `Mode::Inout` | `edit` | Permiso exclusivo |
| `Mode::Consume` | `move` | Transferencia |
| `Type::I64` | `I64` | Anchura/rango sin cambio |
| `Type::Bool` | `Bool` | Separado de entero |
| `Type::Unit` | `()` | Unidad |
| `Type::MockResource` | `MockResource` | Identidad de recurso |
| `Type::MockBlockingResource` | `MockBlockingResource` | Identidad y destructor |
| `Type::Array(t)` | `Array<I64>` | Un argumento, sin longitud en el tipo |
| `Type::Result(t,e)` | `Result<I64, I64>` | Dos argumentos ordenados |
| `WithClause::Simple` | `with no_io, no_alloc` | Vector y orden sin conversión a mapa en AST fuente |
| `WithClause::Operations` | `with {call(no_io)}` | Vector ordenado de OperationLimit |
| `OperationLimit` | `call(no_io, no_alloc)` | name, restrictions, span |
| `Block` | `{ let x = 1; }` | statements y span |
| `Statement::Let { mutable:false }` | Binding: `let x: I64 = 1;` | annotation Some/None, nombre y expresión |
| `Statement::Let { mutable:true }` | Binding: `var x = 1;` | Mismos campos, mutable true |
| `Statement::Assign` | Assignment: `(a)[0] = 2;` | Place sigue Expr; índices y orden intactos |
| `Statement::Return { value:Some }` | `return x;` | Retorno explícito |
| `Statement::Return { value:None }` | `return;` | Distinto de Some(Unit) en árbol fuente |
| `Statement::Value { value:Some }` | `value: x;`, o rama simple `then x` | Entrega de rama, expansión local |
| `Statement::Value { value:None }` | `value:;` reconocido inválido | No pertenece a un programa aceptado |
| `Statement::If` | IfStmt: `if c {} else if d {} else {}` | condition, then_block, otherwise, span |
| `Statement::While` | `while c {}` | condition y body |
| `Statement::Block` | `{ Print(1); }` | Ámbito explícito |
| `Statement::Expr` | `Print(1);` | Expresión descartada; no retorno |
| `Expr::Name` | Primary: `x` | String de nombre y span |
| `Expr::Integer` | IntLiteral: `-01` | String firmado, sin convertirlo a otra grafía |
| `Expr::Bool` | `true`, `false` | bool y span |
| `Expr::Unit` | `()` | No Group vacío |
| `Expr::Array` | `[1, 2]`, `[]` | Vector en orden; el vacío puede necesitar tipo esperado |
| `Expr::Group` | `(x + 1)` | Agrupación escrita conservada en CST/AST fuente |
| `Expr::Unary { mode:Consume }` | `move x` | Operación existente |
| `Expr::Unary { mode:Inout }` | `edit a[0]` | Operación existente |
| `Expr::Unary { mode:Read }` | `F()?` | **Propagación**, peculiaridad de la representación actual; no significa `view F()` |
| `Expr::Binary` | `a + b`, `a - b`, `a < b`, `a == b` | op String en `{+, -, <, ==}`, left, right, span |
| `Expr::Call` | `F(1, y: edit x)` | target y args en orden escrito |
| `Expr::Index` | `a[i]` | target, index, span |
| `Expr::If` | `if c then a else b` | condition, dos Block expandidos, otherwise, span |
| `Else::Block` | `else b` o `else { value: b; }` | Block simple expandido o explícito |
| `Else::If` | `else (if d then a else b)` | Cadena directa por la excepción estructural de §3.2 |
| `Argument { label:None }` | Argument: `1` | value y span |
| `Argument { label:Some }` | Argument: `x: move r` | Etiqueta, expresión y span desde etiqueta |

La representación actual de **else if sentencia** merece una regla explícita: `statement_inner` construye `Statement::If.otherwise = Statement::Expr(Expr::If(...))` para esa continuación, y `Statement::Block` para else ordinario. El parser B conserva ese convenio interno; no imprime `;` después de ese Expr especial ni lo convierte en if expresión con `then`. Solo en esa posición el nodo representa una continuación de sentencia. Un AST construido a mano debe satisfacer este invariante; no se permiten otras Statement arbitrarias como otherwise.

### 4.2. Igualdad normalizada y lista cerrada de cambios

`N(ast)` elimina exclusivamente spans/origen, trivia y Group puramente sintácticos **después de** conservar la distinción contextual necesaria; no elimina contratos, anotaciones, efectos, nombres resueltos, etiquetas, orden ni modos. El parser ya expande los operandos simples de selección con la excepción de §3.2. N **no** equipara `Else::If(e)` con `Else::Block([Value(e)])`: conserva todos los Block y Value explícitos, incluso sin locals, para no alterar limpieza, diagnóstico ni trabajo. El origen del if anidado permanece registrado. Los comentarios pertenecen al CST y condicionan el formato, aunque no formen parte de N.

En el subconjunto F0, el conjunto de variantes **añadidas = ∅; eliminadas = ∅**. No se usa `Expr::Binary` para simular lógica cortocircuitada ni `Mode::Read` para negación. El checker, la IR y el intérprete reciben las mismas operaciones normalizadas y conservan todas sus reglas. Añadir `then` no añade condición ni evaluación. Los marcadores implícitos de rama reciben span del operando escrito; los diagnósticos semánticos señalan ese operando y no una palabra `value` inexistente.

Fuera de F0 se necesitarán nodos para capacidades ya diferidas: declaraciones/tipos nominales y genéricos, campos/métodos, módulos/imports/visibilidad, closures/capturas/procedencias, match/patrones, futuros/await/regiones, generadores/yield/for/control, cadenas/interpolación/decimales y operadores completos. **No son cambios de AST exigidos por migrar F0** ni se implementan con este corte. La única diferencia de dominio expresamente añadida por B respecto de D14 A es el rango inclusivo: cuando exista Range, tendrá extremo `exclusive | inclusive`; el AST actual no tiene Range al que añadir un bit. La selección compacta, modos y nombres no requieren variantes nuevas. Esta es la lista cerrada de cambios propios de B; cada futura capacidad conserva su ADR semántico y su puerta de fase.

La igualdad de nombres históricos y nuevos se prueba con una **biyección de identidades declarada**, no borrando todos los nombres. La migración de nombres públicos es una migración de API, no alfa-renombrado inocuo. El manifiesto de §2.3 enlaza cada grafía B con su identidad intrínseca histórica. Una función de usuario que sombrea `Print` conserva su propia identidad; `print` no resuelve la intrínseca salvo que el usuario declare ese nombre.

Límite de implementación observado: `src/check/names.rs` (`register` y auxiliares de spans) consume AST y tokens, y las tablas de preámbulo/entrada y `src/diagnostic.rs` contienen grafías literales. Por ello no se promete intercambiabilidad byte a byte de structs Rust ni que baste reemplazar un token. La integración debe proporcionar desde frontend/manifiesto las grafías y ubicaciones de nombre/tipo, sin hacer que el checker vuelva a interpretar el orden de parámetros A. Se permiten cambios de tablas y conexiones de origen/representación; **cero cambios a reglas de tipos, propiedad, efectos, lowering o ejecución**. Si se necesita alterar una regla semántica para aceptar B, la equivalencia no está cumplida y la migración se detiene. Tampoco se arregla mediante un segundo parser A oculto detrás de `check`.

## 5. Impresor canónico y formateador

### 5.1. Salida determinista

1. UTF-8 sin BOM, LF; indentación de cuatro espacios; sin tabulaciones ni espacios al final. Programa no vacío termina con un único LF; programa vacío se imprime como cadena vacía. Una línea vacía entre declaraciones superiores, ninguna línea vacía introducida dentro de un bloque. No hay ajuste dependiente del ancho de pantalla.
2. Llave de apertura en la misma línea tras un espacio; bloque vacío `{}`; bloque no vacío: LF tras `{`, una sentencia por línea indentada y `}` alineada. `} else {` o `} else if … {` en una línea. Sin `;` después de un bloque estructural.
3. Para F0, **toda sentencia simple lleva `;` explícito**, aunque el usuario pueda omitirlo por salto. Esto fija una única salida y evita cambios por continuaciones de la siguiente sentencia. Imports, mod, alias, const, requisitos sin cuerpo y campos usan también `;` en salida completa. Variantes y brazos usan coma entre elementos, nunca final. Las fuentes de aceptación con inserción siguen siendo entradas válidas, aunque no sean salida canónica.
4. Un espacio después de `,` y `:`; ninguno antes. Un espacio a ambos lados de `=`, operadores binarios, `->`, `then`, `else` y `and` de cotas; ninguno junto al punto, junto a `< >` de tipo, dentro de `() []` ni antes de una llamada/índice/`?`. Rangos canónicos: `lo..<hi`, `lo..=hi`. Prefijos: `move x`, `edit x`, `not x`, `await x`; signo entero unido a los dígitos.
5. `fn Nombre(parametro: view Tipo) -> Tipo with … {`: modos obligatorios tras `:`. No añadir ni quitar `-> ()` si el AST lo distingue de resultado omitido. Listas de argumentos, parámetros, arrays, tipos y capturas en una línea salvo que contengan bloques o comentarios; no comas finales. Los elementos con bloque se imprimen recursivamente con indentación del contexto actual, nunca desde profundidad cero como hace hoy PRINT `expr` para If.
6. Contrato simple `with no_io, no_alloc`; mapa `with {call(no_io), …}`. Se preservan forma Simple/Operations y orden del AST. Un impresor de **interfaces semánticas** puede materializar un mapa explícito como exige D14 §3.3; esa operación no es el impresor del AST fuente ni se somete a igualdad de sus variantes. No ordenar restricciones para aparentar equivalencia estructural.
7. Modificadores completos en orden `pub async gen fn`, reconociendo la combinación inválida para diagnóstico. Después de parámetros: resultado, `from`, `result_from`, `retains` en orden escrito, `with`, `where`, cuerpo. Atributos en líneas propias antes de la declaración, conservando orden. `self: view` no adquiere un tipo impreso inventado.
8. Una rama se imprime como operando simple **si y solo si** contiene un único Value con operando, no tiene comentario asociado y no es `Else::Block` cuyo Value contiene un If tras quitar Group. Esta última forma conserva su bloque explícito para distinguirla de `Else::If` (§3.2). Si un operando simple de then es otra selección, se agrupa. Las restantes ramas usan bloque con entrega/return explícito. `Else::If` expresión se imprime `else (if … then … else …)`. No se añade `then` a las sentencias if.
9. Conservar paréntesis escritos en el CST; al imprimir AST sin trivia, añadir solo los que requiere la precedencia, asociación o desambiguación contextual. Un hijo binario derecho de igual precedencia necesita agrupación para preservar el árbol; una comparación hija de comparación también. Conservar `(move)(x)` y `(x.campo)()`; no desagruparlos por observar un tipo invocable. Una selección usada en operación superior requiere paréntesis. El CST determina si un comentario impide abreviar una rama.

### 5.2. Formato, nombres e invariantes comprobables

`fmt` solo cambia trivia/puntuación segura y conserva comentarios/literales/nombres. `fmt --check` falla si el texto o la convención de nombres incumple la norma. `fmt --fix-names` es una refactorización resuelta, transaccional, separada del impresor: declaración y referencias/etiquetas por identidad, con comprobación de colisiones, sombreado, rutas portables, exportaciones, serialización y vínculos FFI. Una API pública o campo serializado no se renombra silenciosamente. En particular, `estado` de EX02 conserva su clave JSON. Colisión o dependencia no editable bloquea la transacción, sin sufijos improvisados (D14 §2.3).

Comentarios de línea se asocian al token precedente si comparten línea; en otro caso al siguiente nodo, o al EOF si no lo hay. Los de fin de línea quedan después del `;` con un espacio; los independientes conservan bytes y orden, con indentación del bloque. Un comentario entre tokens obliga a conservar el salto necesario y a no abreviar su rama; no se desplaza a través de `return`/`value:`. El formato no produce un programa «corregido» si hay errores sintácticos: puede presentar diagnóstico/CST parcial, pero no afirmar equivalencia.

Propiedades, para el dominio de AST aceptados cuya impresión canónica en UTF-8 cabe en el presupuesto de bytes de fuente de A2 §13.13 (la misma cota que acepta el lector), y con los invariantes de §4. Las propiedades que usan Format requieren asimismo que su salida quepa en esa cota:

```text
parse(print(ast)) == ast                    // igualdad del AST NORMALIZADO N
N(parse(print(ast))) = N(ast)
print(parse(print(parse(src)))) = print(parse(src))
Format(Format(src)) = Format(src)
N(parse(Format(src))) = N(parse(src))
```

Para un AST aceptado cuya impresión canónica excede ese presupuesto, la impresión sigue siendo correcta conforme a §5.1, pero no es reparseable por el compilador: el reparseo produce NX-E0003/source_bytes, como especifica A2 §13.13. No se exigen en ese caso las propiedades anteriores que reparsean la impresión o el formato; las pruebas de estas propiedades se ejecutan sobre el dominio acotado. Las fixtures de presupuesto se comprueban según §§7.2 y 8.13.

La primera igualdad **no** es `PartialEq` Rust que compara spans: volver a imprimir cambia offsets. Tampoco autoriza borrar Group antes de decidir método/campo o nombre/prefijo. Las pruebas comparan ambas cosas por separado: N conserva operaciones/identidades; el mapa de origen comprueba cada span sobre la nueva fuente. El renombrado explícito compara mediante la biyección registrada. No se reclaman estas propiedades para AST inválidos (`Value(None)`, if expresión sin else) ni para formas completas aún fuera de F0.

## 6. Enmienda cerrada de contratos textuales

### 6.1. Versiones, campos y estabilidad

El identificador B se obtiene por regla: `"f0-0002-b-" + sufijo`, donde sufijo es todo lo que sigue a `f0-0002-` en el perfil vigente de A2. Así `f0-0002-v2` pasa a `f0-0002-b-v2` y `f0-0002-test-flow-global-32-v2` a `f0-0002-b-test-flow-global-32-v2`; no es un selector para mantener el anterior en ejecución. `schema_version = 1`, namespaces `Nexo-proposed-v1`/`F0-0002-sim`, códigos, subcausas y vocabularios de operaciones/roles permanecen, salvo los cambios expresos de esta sección. `effect_model = f0-0002-sim-v1` permanece. Los artefactos históricos conservan su perfil original como evidencia, no como entrada aceptada por el nuevo frontend.

Durante el intervalo hasta completar A2 §17.2, el corte emite `f0-0002-b-v2` como identidad del contrato exigido, aunque el código de partida aún use v1 y omita `budget_scope`, `incomplete_analysis`, `analysis_function` y la cuota global de propiedad. El identificador no acredita conformidad: esos fallos se registran como obligaciones pendientes de I10a, la inferencia/procedencia de I07c y los ticks/spans/reservas de I10b conservan sus esperados normativos. No se introducen esperados alternativos v1, ni se convierten discrepancias de métricas o diagnósticos en implementation_limit para eludirlas. §7.3 exige distinguir estas obligaciones de la equivalencia de superficie.

Lista cerrada de cambios de valores públicos de A2 §§13.6–13.17:

| Ubicación | Nuevo valor o regla literal B |
|---|---|
| Perfil normal, incluido RepairValidation | `"f0-0002-b-v2"`; perfiles reducidos según tabla siguiente |
| §13.6 `data.entry` de NX-E0112 | `"Main"` |
| §11.1/§13.13 NX-E0003 | Conservar budget_scope, incomplete_analysis y analysis_function de A2 v2; primario local dentro de esa función, con la restricción adicional y orden de B-D1 en §8.13. |
| §13.6 `feature_id` y `pending_scope` | Retirar `legacy_move` y `migration`; añadir `implementation_limit` y sus cinco pending_scope de §7.1, conservando 30 entradas en §3.6 |
| §13.14 FIRST y filas sintácticas | Listas y fuentes de §§6.2–6.3 |
| §13.15 parcial transferido | `related_spans.context` sobre `move` o `Drop`, con longitud de la grafía B |
| §14.11 R0102/context | Argumento completo, incluidas etiqueta y `edit` (antes `inout`); recalcular el rango completo, no solo el selector. D03/§8.12 fija el caso sin etiqueta. |
| §13.15 consumo y llamadas | Spans sobre `move p`, `Drop(p)` y los destinos B; **operation sigue `value.consume`, `borrow.inout`, `value.drop`** |
| §13.17 tipos serializados | `I64`, `Bool`, `()`, `MockResource`, `MockBlockingResource`, `Array<T>`, `Result<T,E>` sin espacios; `Never` solo interno |
| §13.17 S16…S21 | `I64/Bool`; `I64/Array<I64>`; `I64/Bool`; `domain:copyable_data/MockResource`; `I64/Result<I64,I64>`; `domain:copyable_data/Result<MockResource,I64>` |
| §13.17 rutas indeterminadas | `$.element`, `$.ok`, `$.err` sin cambio; el ejemplo anotado usa `Result<I64,I64>` |
| §13.7 y §13.8 fuentes de presentación | `Main`, `Print`, tipos B; ejemplos concretos abajo |
| §13.11 corpus migrado | Nombres B; offsets recalculados en fuente B, nunca sumados globalmente a offsets A |

Los ocho perfiles reducidos de A2 §§11.4/12.4 y COD «Perfil de análisis de F0-0002»/«Runtime experimental de F0-0002» conservan sus cuotas y reciben identidades distintas; no se colapsan en el perfil normal:

| Perfil B | Única cuota distinta de `f0-0002-b-v2` |
|---|---|
| `f0-0002-b-test-types-32-v2` | `type_solver_steps = 32` |
| `f0-0002-b-test-flow-32-v2` | `ownership_flow_steps` local = 32; global conserva 4.000.000 |
| `f0-0002-b-test-flow-global-32-v2` | `ownership_flow_steps` global = 32; local conserva 1.000.000 |
| `f0-0002-b-test-effects-32-v2` | `effect_fixpoint_steps = 32` |
| `f0-0002-b-test-ir-32-v2` | `ir_items = 32` |
| `f0-0002-b-test-frames-8-v2` | `call_frames = 8` |
| `f0-0002-b-test-cells-64-v2` | `live_value_cells = 64` |
| `f0-0002-b-test-observations-64-v2` | `observation_bytes = 64` |

`nexo explain NX-E0006` devuelve **30 entradas**, una por feature_id de §3.6, ordenadas por bytes. Conserva `subcause=null`, `explanation_id=NX-E0006`, `availability=in_profile` y `feature_ids=[ID]`; `example_profile_id=f0-0002-b-v2`. `negative_source`/`positive_source` son la primera fila B por número de §8.7 para ese ID y su edición completa: `unsupported_type` usa F2-F29, `methods` F2-F23. Se elimina legacy_move y se incorpora implementation_limit; sus ejemplos son `fn Main()->Result<(),I64>{return Ok(());}` y `fn Main(){}`, con pending_scope I08 y primario `Result<(),I64>` en `[11,25)`. El ejemplo negativo de Result es válido solo hasta implementar I08; desde entonces deja de ser un rechazo y debe sustituirse por una capacidad aún pendiente de §7.1. Su explicación enumera las cinco etapas pendientes de §7.1; no acusa una prohibición del lenguaje. NX-E1301 conserva `availability=historical`, `example_profile_id=null` y sus ejemplos se identifican como históricos, no fuentes B aceptables. El esquema, orden y errores de uso de explain de A2 §13.10 no cambian.

`prelude:<nombre>` identifica la identidad estable del manifiesto: se conservan `prelude:print`, `prelude:mock_open`, etc., aunque su grafía fuente sea `Print`/`MockOpen`. No son nombres aceptados por el parser. Las identidades `function:/parameter:/binding:` y préstamos/temporales vuelven a usar offsets de la fuente B; no son estables a un cambio textual. Las identidades de tipos serializadas usan la grafía B. Se conserva `active_permission = read | inout`, `requested_access = read | write | consume | drop | borrow_inout`; son vocabulario semántico, no residuos de sintaxis aceptada.

Salvo el rechazo temporal explícito implementation_limit de §7.1, no cambian diagnósticos semánticos, prioridades, etapas, número de causas, selección de testigos, orden de argumentos, presupuestos, resultados, eventos ni trabajo de A2 por cambiar una grafía. La única fila que deja de ser negativa es F2-F01, porque `move` ahora es válido. Se conservan `missing_main` e `invalid_main_signature` como subcausas estables, aunque `data.entry` sea `Main`.

### 6.2. FIRST y tokens esperados de F0

Listas literales ordenadas por bytes; Name incluye los contextuales, pero se enumeran también los prefijos explícitos en los estados que los admiten. Estas listas **no** incluyen construcciones diferidas como si fueran aceptadas.

```text
ExprStart = ["(","-","IntLiteral","Name","[","edit","false","if","move","true"]
TryStart = ["(","-","IntLiteral","Name","[","edit","false","move","true"]
PostfixStart = ["(","-","IntLiteral","Name","[","false","true"]
BlockNext = ["(","-","IntLiteral","Name","[","edit","false","if","let","move","return","true","var","while","{","}"]
ParameterListFirst = [")","Name"]
ParameterFirst = ["Name"]
ParamModeFirst = ["edit","move","view"]
TypeStart = ["(","Array","Bool","I64","MockBlockingResource","MockResource","Result"]
GroupFirst = ["(",")","-","IntLiteral","Name","[","edit","false","if","move","true"]
ArrayFirst = ["(","-","IntLiteral","Name","[","]","edit","false","if","move","true"]
ElseStmtFirst = ["if","{"]
BranchFirst = ["(","-","IntLiteral","Name","[","edit","false","move","true","{"]
ModuleNext = ["EOF","fn"]
FunctionSuffix = ["->","with","{"]
BinaryAccepted = ["+","-","<","=="]
OperationFirst = ["call","cancel","copy","create","drop","resume"]
OperationListFirst = ["call","cancel","copy","create","drop","resume","}"]
RestrictionFirst = ["no_alloc","no_block","no_heap_alloc","no_io","no_suspend","unrestricted"]
WithFirst = ["no_alloc","no_block","no_heap_alloc","no_io","no_suspend","unrestricted","{"]
```

Después de un parámetro sin modo: esperado ParamModeFirst, no TypeStart. Después del modo: TypeStart. Después de `if condición` en expresión: `["then"]`; tras `then` o `else`: BranchFirst. En if sentencia, después de else: ElseStmtFirst. La ausencia de else tras rama completa usa NX-E1302/missing_else y no una lista genérica. `value:` con salto conserva su diagnóstico específico.

Para los restantes truncamientos se conserva la producción comprometida más interna y FIRST de su resto, incluida nulabilidad/EOF, sin elegir una alternativa por el token erróneo. Empate: orden de alternativas EBNF. Contexto `grammar.P`, con P de §3.1 o las producciones de contrato allí incluidas; los contextos especiales de §6.3 prevalecen. `recognized_prefix` va del primer token comprometido al comienzo del inesperado, incluyendo trivia; si no se consumió token, es vacío. Program empieza en cero. `consume_place/inout_place` conservan sus identificadores históricos porque describen operaciones normalizadas; no cambian a partir de traducción de mensajes.

### 6.3. Filas sintácticas literales que sustituyen A2 §13.14

Fuentes sin LF final. Para NX-E0002: `stage=parse`, `operation=syntax.parse`, `expected_tokens_kind=admissible_set`, token.span=primary_span, `repair=[]`, `data={}`, `subcause=null`. Listas nombradas se expanden literalmente según §6.2. EOF: kind `eof`, text `""`. El resto de campos comunes usa §6.1/A2 §13.6.

| Fuente / ID | Token, span | Contexto | Esperados | Prefijo |
|---|---|---|---|---|
| `fn` | EOF `[2,2)` | `function_name` | `["Name"]` | `[0,2)` |
| `fn Main(` | EOF `[8,8)` | `parameter_list` | `[")","Name"]` | `[7,8)` |
| `fn Main(){` | EOF `[10,10)` | `block_item` | BlockNext | `[9,10)` |
| `fn Main(){let a=` | EOF `[16,16)` | `binding_initializer` | ExprStart | `[10,16)` |
| `fn Main(){let a=-` | EOF `[17,17)` | `signed_integer` | `["IntLiteral"]` | `[16,17)` |
| `fn Main(){let a=[1,` | EOF `[19,19)` | `array_element` | ExprStart | `[16,19)` |
| `fn Main(){let a=1;} garbage` | Name `garbage` `[20,27)` | `module_item` | `["EOF","fn"]` | `[0,20)` |
| `// expect: NX-E0002` + LF + `fn Main() {} ignored` | Name `ignored` `[33,40)` | `module_item` | `["EOF","fn"]` | `[0,33)` |
| Mismo con CRLF | Name `ignored` `[34,41)` | `module_item` | `["EOF","fn"]` | `[0,34)` |
| F2-S22 `fn Main(){let match=1;}` | Keyword `match` `[14,19)` | `binding_name` | `["Name"]` | `[10,14)` |
| F2-S27 `fn Main(){let x=1*2;}` | Symbol `*` `[17,18)` | `unsupported_binary_operator` | BinaryAccepted | `[10,17)` |
| F2-S28 `fn Main(){let x=1!=2;}` | Symbol `!=` `[17,19)` | `unsupported_binary_operator` | BinaryAccepted | `[10,17)` |
| F2-S30 `fn F(x:edit I64){} fn Main(){F(edit 1);}` | Integer `1` `[36,37)` | `inout_place` | `["Place"]` | `[31,36)` |
| F2-S31 `fn Main(){let x=move 1;}` | Integer `1` `[21,22)` | `consume_place` | `["Place"]` | `[16,21)` |
| F2-S32/S33/S34 `fn Main(){let x=1>2;}` / `<=` / `>=` | Symbol, `[17,18)` / `[17,19)` / `[17,19)` | `unsupported_binary_operator` | BinaryAccepted | `[10,17)` |
| F2-K02 `fn Main() ;` | Symbol `;` `[10,11)` | `function_suffix` | `["->","with","{"]` | `[0,10)` |
| `fn F(x:I64){}` | Name `I64` `[7,10)` | `grammar.ParamMode` | `["edit","move","view"]` | `[7,7)` |
| `fn F(x:view){}` | Symbol `)` `[11,12)` | `grammar.Type` | TypeStart | `[11,11)` |
| `fn Main(){let x=if true{value:1;}else{value:2;};}` | Symbol `{` `[23,24)` | `grammar.IfExpr` | `["then"]` | `[16,23)` |
| `fn Main(){let x=0..1;}` | Symbol `..` `[17,19)` | `range_endpoint` | `["..<","..="]` | `[16,17)` |

Tres filas de los operadores de §3.2, con los campos comunes NX-E0002 anteriores:

| Fuente B literal | Token y primario | parser_context | expected_tokens | recognized_prefix |
|---|---|---|---|---|
| `fn Main(){let x=not true;}` | keyword not `[16,19)` | unsupported_unary_operator | ExprStart | `[10,16)` |
| `fn Main(){let x=true and false;}` | keyword and `[21,24)` | unsupported_binary_operator | BinaryAccepted | `[10,21)` |
| `fn Main(){let x=true or false;}` | keyword or `[21,23)` | unsupported_binary_operator | BinaryAccepted | `[10,21)` |

Positivos aislados: quitar `not `; sustituir `and` o `or` por `==`. Los esperados se expanden literalmente desde §6.2, no desde el parser bajo prueba.

En F2-S32…S34, añadir LF inmediatamente antes del operador aumenta en uno el comienzo/fin del operador y el final del prefijo; no cambia código ni crea terminador. CRLF aumenta dos. Los ejemplos de funciones truncadas conservan el mismo número de bytes al cambiar main→Main.

NX-E0001 conserva `stage=lex`, `operation=syntax.lex`, `parser_context=lexical_token`, `expected_tokens=["valid_token"]`, kind `lexical_category`, prefijo `[0,start)`, `repair=[]`, `data={}`. F2-K01 es `fn Main() { £ }`: `£` invalid `[12,14)`, prefijo `[0,12)`. El positivo es `fn Main() { }`.

Las cinco fuentes de A2 §14.8 quedan literalmente como sigue, sin LF final. Cada positivo sustituye solo el token marcado por `+`. No se añaden división/resto al checker para hacer pasar estas filas.

| ID | Fuente B | Código / contexto | Token, span y prefijo | Esperados |
|---|---|---|---|---|
| F2-LX01 | `fn Main(){let x=1⟦/⟧2;}` | NX-E0002 / `unsupported_binary_operator` | symbol `[17,18)`, prefijo `[10,17)` | `["+","-","<","=="]` |
| F2-LX02 | `fn Main(){let x=1⟦%⟧2;}` | NX-E0002 / `unsupported_binary_operator` | symbol `[17,18)`, prefijo `[10,17)` | `["+","-","<","=="]` |
| F2-LX03 | `fn Main(){let x=1⟦!⟧2;}` | NX-E0001 / `lexical_token` | invalid `[17,18)`, prefijo `[0,17)` | `["valid_token"]` |
| F2-LX04 | `fn Main(){let x=1⟦&⟧2;}` | NX-E0001 / `lexical_token` | invalid `[17,18)`, prefijo `[0,17)` | `["valid_token"]` |
| F2-LX05 | `fn Main(){let x=1⟦\|⟧2;}` | NX-E0001 / `lexical_token` | invalid `[17,18)`, prefijo `[0,17)` | `["valid_token"]` |

La barra inversa de la última celda escapa la tabla Markdown y no es un byte de fuente: el único token inválido es `|`. NX-E0002 usa etapa parse y `admissible_set`; NX-E0001, etapa lex y `lexical_category`.

### 6.4. Reparaciones, ejemplos JSON y spans semánticos

El vocabulario emitible ordinario de Repair.id sigue siendo **solo `insert_terminator`**. Para NX-E0005: expected `[";"]`, kind `canonical_insertion`, contexto `statement_terminator`, prefijo desde comienzo de sentencia hasta token inesperado, related `preceding_expression` sobre la expresión completa. La edición inserta en el **final de la expresión**, antes de trivia, no en el comienzo del siguiente token si hay espacios/comentario entre ambos. Estado `proposed`, validation null, expected_text `""`, replacement `";"`. El modo explícito de recomprobación puede declarar `applicable` solo después de comprobar todo el programa (A2 §§13.6 y 13.15). No se ofrece edición automática de `return/value:` ni de extremo de rango.

Sustitución literal del ejemplo A2 §13.7, archivo `terminador.nx`, fuente `fn Main(){let x=1}` sin LF final:

```json
{
  "schema_version": 1,
  "diagnostic_namespace": "Nexo-proposed-v1",
  "profile_id": "f0-0002-b-v2",
  "code": "NX-E0005",
  "subcause": null,
  "rule": "NX-E0005",
  "explanation_id": "NX-E0005",
  "severity": "error",
  "stage": "parse",
  "message": "Falta un terminador de sentencia.",
  "operation": "syntax.parse",
  "primary_span": {"file":"terminador.nx","start":17,"end":18},
  "related_spans": [{"role":"preceding_expression","span":{"file":"terminador.nx","start":16,"end":17}}],
  "cause_chain": [],
  "feature_id": null,
  "unexpected_token": {"kind":"symbol","text":"}","span":{"file":"terminador.nx","start":17,"end":18}},
  "expected_tokens": [";"],
  "expected_tokens_kind": "canonical_insertion",
  "parser_context": "statement_terminator",
  "recognized_prefix": {"file":"terminador.nx","start":10,"end":17},
  "repair": [{"id":"insert_terminator","status":"proposed","description":"Insertar un terminador explícito.","edits":[{"span":{"file":"terminador.nx","start":17,"end":17},"expected_text":"","replacement":";"}],"validation":null,"consequences":["Termina el binding antes del cierre del bloque."]}],
  "data": {}
}
```

Los otros dos ejemplos completos de A2 §13.7 cambian exclusivamente perfil/grafías: `decimal.nx`=`fn Main(){let x=0.5;}` conserva NX-E0006/float_literal `[16,19)`, todos sus null/listas vacías y `pending_scope=later_adr`; el ejemplo R0101 usa `Print` y `Main`, y conserva longitud de nombres y todos los spans del ejemplo. A2 §13.8 muestra esas mismas grafías en la línea de fuente; su plantilla de texto, columnas Unicode, números de línea, exit status y reglas de escape no cambian. `nexo explain` usa las fuentes B y las listas de este ADR, sin cambiar la identidad del código.

Los primarios de todas las fuentes publicadas de §14 se transcriben con `⟦…⟧` en §8; esos delimitadores no son bytes de fuente. La tabla de offsets de §8 es parte del esperado, no salida del compilador. Tipos/nombres relacionados se localizan por la misma declaración/nodo causal en la nueva fuente. En particular, la anotación del parámetro comienza **después del modo**, no después del primer `:`; el span de su nombre sigue cubriendo solo el identificador. No usar el span completo de Parameter como sustituto del nombre.

Las cuatro copias de A2 §13.11 se fijan literalmente así; cada bloque contiene un LF entre comentario y programa y puede conservar el LF final sin cambiar los primarios:

```nexo
// expect: NX-E0101
fn Main() { { let r = MockOpen(1); } Inspect(⟦r⟧); }
```

```nexo
// expect: NX-E0105
fn Main() { let value = [[1]]; ⟦value⟧[0][0] = 2; }
```

```nexo
// expect: NX-E0107
fn Main() { let value = ⟦9223372036854775808⟧; }
```

```nexo
// expect: NX-E0201
fn Main() { return; let r = MockOpen(1); let duplicate = ⟦r⟧; }
```

| Archivo de la copia | Primario LF | Primario CRLF |
|---|---|---|
| `escape_scope.nx` | `[65,66)` | `[66,67)` |
| `immutable_write.nx` | `[51,56)` | `[52,57)` |
| `overflow_literal.nx` | `[44,63)` | `[45,64)` |
| `unreachable_resource_copy.nx` | `[77,78)` | `[78,79)` |

Los cambios de escape_scope y unreachable_resource_copy incluyen el byte menos de `MockOpen` respecto de `mock_open`; Inspect conserva la longitud. Los positivos respectivos sacan la declaración r al ámbito exterior, cambian let value a var value, usan el máximo I64 representable y transfieren mediante `move r`. Cada uno se recomprueba completo. No se reutilizan offsets de un comentario sin el prefijo NX-.

### 6.5. Enmiendas exactas al catálogo y a las filas de corpus

COD sigue siendo el registro único integrado de códigos. Esta sección fija su superficie normativa B y la frontera de capacidades pendientes, sin crear otro espacio de números.

| Entrada/fila de COD o A2 | Texto/valor operativo B |
|---|---|
| NX-E0001 | Token no reconocido por el inventario de §2.4; `/ %` dejan de ser fallos léxicos, `! & |` siguen si están aislados |
| NX-E0002 | Gramática B §3; FIRST §6.2; errores de retiradas y formas A no se aceptan como compatibilidad |
| NX-E0005 | Mismos código y reparación `insert_terminator`; nueva fuente B y offsets §6.4/§8 |
| NX-E0006, alcance de A2 | `legacy_move` retirado; `range` sobre `..<`/`..=`; las restantes entradas conservan significado y usan fuentes B |
| NX-E0105, descripción de receptor/patrón | «receptor edit y match edit»; misma regla de permiso |
| NX-E0107, grafía del tipo | «Literal fuera del rango I64 del corte» |
| NX-E0112 (A2) | Entrada `Main`, subcausas `missing_main/invalid_main_signature` sin cambio |
| NX-E0202, descripción de orígenes | Transferencia `move`, intrínseca `Drop` y extracción consumidora; mismas subcausas |
| NX-E0203 y K10…K13 históricos | No emitibles como limitaciones de F0 B; la conciliación A1→A2 conserva su significado histórico, sin reintroducirlas |
| NX-E1301/implicit_closure_body | Número reservado al historial de D11/D13; no emitible para B, que rechaza ambos cuerpos con flecha por NX-E0002 en el cuerpo completo (§3.4) |
| NX-E1302 | Misma lista de cinco subcausas; missing_else se reconoce sobre IfExpr con `then`; ramas simples usan span del operando expandido |
| COD K01/K02 usados por A2 | Copias B F2-K01/F2-K02 según §6.3; las filas históricas F0-0001 no cambian |
| COD ejemplos/explicaciones operativas restantes | `inout`→`edit`, `consume`→`move`, `borrow` de captura→`view`, nombres de APIs según §2.3, parámetros `nombre: modo Tipo`, if expresión con `then`; no se modifican identificadores de regla/operation/role |

Esta es la lista cerrada de **reglas** cambiadas; todo código restante de COD conserva significado, propietario, subcausas y prioridades. No se reutiliza un número retirado. Los ejemplos históricos de COD, otros ADR y baseline conservan sus bytes bajo inventario histórico, pero dejan de ser instrucciones de sintaxis vigente. Los ejemplos operativos que se extraigan de ellos se imprimen en B desde su AST, con su propia identidad de copia y expectativas de superficie.

En **COD integrado**, «Registro canónico»/«Precisiones de F0-0002» usan Main para entrada; «Feature IDs» aplica §3.6 (retira legacy_move y añade implementation_limit), range usa `..<`/`..=`, receiver_mode usa `edit/move`, partial_transfer usa `move`/`Drop`, unsupported_type usa `U32/U64/F32/String/app.Error`. «Operadores y proyecciones» conserva contextos estables y agrega los operadores excluidos de §3.2. «Perfil de análisis»/«Runtime experimental» transforman por la regla de §6.1 el perfil normal y los ocho reducidos, sin cambiar cuotas. «Limitaciones históricas» conserva NX-E0203 como no emitible: el binding completo se transfiere con `move`, y proyecciones/receptores conservan partial_transfer/receiver_mode. «Explicaciones» usa las 30 entradas y fuentes B; «Datos estructurados» aplica §6.1, incluidas las coordenadas de implementation_limit. Ninguna otra regla de códigos, presupuestos u observación cambia.

La lista cerrada de **filas de A2 §14 afectadas textualmente** es: F2-S01…S34; F2-P01…P11; F2-PP01…PP06; F2-R01…R03; F2-B01…B09; F2-E01…E06 y las tres variantes de E05; F2-W01…W07; F2-F01…F33; F2-L01…L07 y variantes; F2-LX01…LX05; F2-RT01; fuentes numeradas 1…6 de §14.5; igualdad estructural y entradas de §14.1; los tres positivos de §14.9; copias F2-K01/K02; T01–T07 (TD01–TD07 de A2 §14.10) y D01–D08 (DG01–DG08 de A2 §14.11), con sus variantes y relaciones. Todas están materializadas en §8; §8.12 transcribe las quince filas nuevas y §8.13 B-D1. Las filas 7/8 de §14.5 son recetas/entrada mock, no nuevas fuentes literales: se migran sus programas de §§11/12 desde AST y se conservan sus presupuestos y JSON. Las identidades de esas filas no se pierden ni se cuentan como programas vacíos.

Cambios de veredicto/contrato textual dentro de ese conjunto: F2-F01 pasa a positivo; F2-F04 usa `..<`; F2-LX01/LX02 pasan de léxico a sintáctico; F2-B01 conserva missing_else con `then` mediante el reconocedor inválido; F2-L03 conserva deliberadamente función `value` minúscula para probar el identificador contextual y se excluye solo de la comprobación de estilo de esa fixture. El resto conserva el diagnóstico semántico/resultado publicado. No se convierte ningún negativo de propiedad en fallo de sintaxis accidental. `diagnostic_unicode_comment_location` y las fuentes de `nesting_limit` conservan sus offsets al sustituir únicamente `main` por `Main` (cuatro bytes en ambos); cambiar namespace/código no desplaza su fuente.

## 7. Migración en un solo corte

### 7.1. Secuencia y frontera del cambio

1. Congelar un inventario por archivo y sitio de fuente de todo el repositorio: corpus `.nx`, literales Rust/Python, plantillas, documentación operativa, ejemplos de CLI/LSP, respuestas esperadas y herramientas. El censo D14 §10.4 es un antecedente acotado, no un total actual ni una excusa para omitir fragmentos sin `fn`. Registrar el commit de referencia y hashes de blobs; no usar el árbol de otro agente como oráculo mutable.
2. Extraer los AST **solo de fixtures admitidas por `src/frontend/parse.rs` de F0-0002** y conservar entradas/resultados/diagnósticos/IR independientes. Renombrar símbolos por identidad con §2.3, resolviendo colisiones. Los 15 archivos de `corpus/` heredados de F0-0001 se **reescriben a mano como literales B**: no se intenta extraer sus AST con ese parser, que rechaza su `move` como legacy_move. Los negativos sintácticos del frontend nuevo se representan por positivo pareado y mutación aislada o CST parcial; no se exige un AST válido del negativo.
3. Sustituir lexer, parser, impresor y tablas de grafías/origen por B, con las variantes de §4. Comparar las fixtures de F0-0002 contra sus contratos previos antes de eliminar A. Una referencia temporal de comparación no forma parte del árbol final ni de la distribución; no queda un parser alternativo.
4. Regenerar desde AST con el impresor B las fuentes analizables de F0-0002 y sus plantillas. Se exceptúan el corpus heredado (literales manuales), las pruebas irreducibles de léxico/trivia y las filas publicadas del ADR. En los aproximadamente 1.100 sitios embebidos, obtener los offsets con un **ayudante de escaneo de bytes escrito en los tests, independiente del impresor**: localiza lexemas y ocurrencias declaradas en los bytes finales, sin llamar a lexer/parser/print del producto ni leer su mapa NodeId→span. Cuenta UTF-8 original y preserva CRLF. Las filas publicadas en ADR se transcriben a mano y se comprueban contando bytes; el mapa del impresor solo puede ser objeto de comparación, nunca el único oráculo. Oráculos de AST/IR/resultado proceden de los contratos, no del código probado.
5. Reaplicar a los positivos B las mutaciones léxicas/sintácticas aisladas; actualizar FIRST, modos, then, trivia y spans desde §§6/8. Materializar LF/CRLF/CR y EOF antes del escaneo de bytes. Las filas manuales también tienen positivos y negativos que prueban su causa, no un rechazo incidental de otra etapa.
6. Comparar resolución, IR, diagnósticos, resultado, efectos, limpieza y trabajo donde la capacidad ya existe. **Ninguno de los 15 archivos requiere implementation_limit**: el pipeline actual, con la grafía equivalente que admite A2, compila y ejecuta o rechaza con el código esperado los 15 archivos de corpus; también admite la traducción de los 10 tests que ejecutan programas en tests/corpus.rs (`positive_corpus`, `nested_copy_output`, `transfer_has_one_owner_and_exact_cleanup`, `early_return_unwinds_scopes_in_reverse_order`, `recoverable_failure_and_partial_initialization`, `explicit_close_and_failure_do_not_double_close`, `indexing_and_overflow_are_checked_in_every_build`, `assignment_bounds_preserve_cleanup`, `assignment_checks_negative_bounds_after_rhs` y `work_budget_exhaustion_closes_resources`), con salidas y eventos coincidentes y los códigos esperados, incluidos R0101, R0102 y R0301, salvo `work_budget_exhaustion_closes_resources`: el pipeline nuevo emite R0002 con `runtime_budget_kind=live_value_cells`, conforme a A2 §§11.3/13.5; durante la migración se reescribe el esperado actual R0001 a R0002. La comprobación se repite sobre B al implementar el corte. «Rechazado: aún no soportado» es una salvaguarda para nuevas fixtures que realmente requieran I08–I11, no el estado de esos 15 archivos. Tales fixtures mantienen el esperado semántico normativo y el rechazo temporal NX-E0006/implementation_limit con su etapa pendiente; el rechazo nunca se cuenta como ejecución satisfactoria ni como esperado alternativo de las métricas de §6.1. Se conserva la trazabilidad en tests/corpus.rs y tests/cli.rs.
7. Eliminar físicamente en este mismo corte `src/syntax.rs`, `src/runtime.rs`, la sección heredada de `src/check.rs`, `nexo::compile`, CheckedProgram y Diagnostic heredados de `src/lib.rs`, y sus dependencias de usuario (§7.2). Reescribir conjuntamente biblioteca y CLI: `nexo check` = parse B + `check_proposed` sin exigir entrada + lowering del subconjunto disponible + `verify_program`; `nexo run` usa el mismo camino y la comprobación de entrada, `compile_scalar`/`run_scalar` y la IR verificada. Refactorizar la entrada común para que `check` de una biblioteca sin Main no herede el `run_entry=true` hoy interno a compile_scalar ni duplique análisis. Verificar también inmediatamente antes de ejecutar. No se mantiene `nexo::compile` como alias ni wrapper del prototipo; no hay selector, fallback, segundo flujo, ni excepción de retirada en I11.

**CLI durante el intervalo.** Hasta I11, `check` y `run` admiten exactamente un argumento de archivo y ninguna opción. `--profile`, `--format` y `--mock-env`, con valor separado o `=`, faltante, repetido o en cualquier posición, se rechazan antes de leer el archivo: stdout vacío, exit 2 y stderr exactamente `nexo-driver:invalid_arguments` seguido de un LF. La misma respuesta rige para comando, aridad u opción inválidos; sin texto de uso ni diagnóstico NX-E, conforme a A2 §§12.4/13.8. `explain` conserva su interfaz y su `--format` de A2 §13.10. Los perfiles reducidos se prueban mediante el arnés interno hasta integrar las opciones de A2 §12.4 en I11.

La lectura conserva el límite previo a decodificación de A2 §13.13: exceso de fuente produce NX-E0003/source_bytes y exit 1; archivo inexistente, ilegible o UTF-8 inválido produce NX-E0004 y exit 2, con los campos de A2 §13.6 y sin invocar el parser. `check` satisfactorio devuelve exit 0, stdout `nexo-check:ok` más LF y stderr vacío; `run` conserva payloads, eventos, resultado y salidas 0/1/2 de A2 §12.4 para las capacidades disponibles. Los diagnósticos usan stderr y la presentación A2 §13.8; los fallos internos controlados dan exit 2, sin disfrazarse de exclusión del lenguaje.

**Límite estable de implementación.** Se usa el diagnóstico existente **NX-E0006** (`subcause=null`, `rule=explanation_id=NX-E0006`, namespace Nexo-proposed-v1, severity error, perfil B vigente), con `feature_id=implementation_limit`. Es la trigésima entrada de §3.6; conserva las otras 29. Se reconoce sobre AST/tipos resueltos, nunca por patrón del texto. `stage=types`, `operation=types.check`; primario sobre la primera construcción que necesita la capacidad pendiente, o EOF vacío para una petición del arnés interno sin construcción fuente. `data` contiene únicamente `pending_scope` de la tabla; related_spans, cause_chain, repair y expected_tokens vacíos, campos sintácticos y expected_tokens_kind nulos. `message` es exactamente `Capacidad pendiente de implementación en {pending_scope}.`, sustituyendo las llaves por el valor literal; exit 1, sin salida de programa ni eventos. Se presenta con A2 §13.8: por ejemplo el Result de §6.1 comienza `NX-E0006:1:12: Capacidad pendiente de implementación en I08.` más LF. Si hay varias capacidades, se elige primero el menor inicio fuente y, para el mismo sitio, I08, I09, I10a, I10b, I11a. Un error ya determinado en una barrera anterior conserva su prioridad. No se convierte cualquier InvalidIr/pánico/fallo interno en este diagnóstico; esos son defectos. Se retira la variante OutsideScalarSlice: la frontera de capacidades se comprueba antes de bajar la IR.

| Capacidad pendiente | `data.pending_scope` | Sitio y criterio de activación |
|---|---|---|
| Result, Ok/Err, `?`, TryAdd y firmas Result de entrada | `I08` | Tipo Result o construcción correspondiente; Main con resultado unidad puede ejecutarse; las demás firmas incompatibles conservan NX-E0112. Result<(),I64> queda pendiente de I08, sin ampliar la lista de A2 §4.4. |
| Contratos y comprobación transitiva de efectos | `I09` | Cláusula with escrita o primitiva que exige el manifiesto aún pendiente, incluida `MockUnknown` (A2 §17.2, I09); su llamada requiere NX-E0006/implementation_limit con pending_scope I09. No se acepta un contrato sin verificarlo. |
| Perfiles reducidos y garantías completas de análisis | `I10a` | Solicitud explícita del arnés interno de un perfil aún no disponible; no rechaza programas ordinarios por necesitar análisis ni sustituye fallos de cuotas o metadatos. La CLI rechaza la opción con exit 2. |
| Cuotas completas de ejecución, contabilidad y reservas de observación | `I10b` | Solicitud del arnés interno de un perfil o protocolo aún no disponible; el subconjunto ordinario conserva sus guardas, sin anunciar conformidad completa con §11 de A2 ni sustituir discrepancias de ticks/reservas por este diagnóstico. La CLI rechaza la opción con exit 2. |
| Interfaz mock de entrada y tres flujos integrados | `I11a` | Primera llamada de la interfaz de A2 §12 aún no ejecutable: `MockReadRecords`, `MockWriteI64`, `MockRequest` y `MockReply` requieren NX-E0006/implementation_limit con pending_scope I11a (A2 §17.2), aunque I08 ya admita sus tipos Result. I11b es integración de evidencia, no una capacidad nueva que justifique rechazar programas. |

Al completar cada capacidad deja de emitirse su rechazo temporal y se ejecutan sus expectativas de A2; la regla NX-E0006 y sus campos no se reutilizan para un error semántico. El ejemplo Result de explain de §6.1 es un límite temporal solo hasta I08; después se sustituye por una capacidad aún pendiente de esta tabla. Cuando no quede ninguna, el ejemplo pasa a histórico.

**Regresión de aceptación intencional.** El pipeline actual acepta y ejecuta `fn main() with no_io{}` sin comprobar efectos. Su fuente B `fn Main() with no_io{}` se rechaza hasta I09 con NX-E0006/implementation_limit, pending_scope I09, primario de la cláusula with `[10,20)`, mensaje `Capacidad pendiente de implementación en I09.`, exit 1 y sin observaciones. El positivo sin cláusula es `fn Main(){}`. No se atribuye esta pérdida de aceptación a una mera traducción de grafía ni se conserva la aceptación sin verificar el contrato.

**Sustitución de guardas internas de `src/ir/lower.rs`.** La profundidad se mide con el modelo publicado, no con la pila de llamadas auxiliares del lowering. Cada exceso estructural produce NX-E0003/analysis.limit con `budget_scope=compilation`, `incomplete_analysis` igual al `stage` de cada fila (`parse` / `types`), `limit=128`, `observed=129`, subcause null y sin feature_id, campos sintácticos, relaciones ni cadena; el perfil es B. No produce implementation_limit.

| Guarda actual | Sustituto exigido |
|---|---|
| `lowering depth` (`depth >= 128` en recorrido de sentencias) | Retirar el contador auxiliar como límite de lenguaje; preservar las comprobaciones compartidas de syntax_nesting (activaciones Block/Expr/Type) y expression_tree_depth (altura de expresión). Si se excede la primera, NX-E0003 en parse, budget_kind syntax_nesting y token de activación de A2 §13.13; si se excede la segunda, NX-E0003 en parse, budget_kind expression_tree_depth y nodo candidato completo. Toda fuente dentro de ambas cotas debe bajar, incluidos los 127 términos compuestos de §13.13. |
| `expression depth` (`depth >= 256`) | Misma altura expression_tree_depth de parser y lowering, no una segunda cuota de 256: NX-E0003 en parse, budget_kind expression_tree_depth y span completo del candidato según A2 §13.13. |
| `array type depth` (`depth > 128`) | NX-E0003 en types, budget_kind array_type_depth, span del tipo escrito completo o de la expresión cuyo tipo se sintetiza; prevalece sobre value_type_depth cuando ambos se exceden (A2 §§11.1/13.13). |
| `assignment root` | En F0, Assign solo admite Place con raíz Name, tras quitar Group e Index; `place_parts` ya garantiza esa raíz. Eliminar la rama redundante o tratar su violación como InvalidIr controlado, exit 2: no representa una capacidad pendiente. Una fuente con destino inválido conserva su diagnóstico de parser; no se convierte en implementation_limit. |
| `nonlocal place` | Aplicar Place y los rechazos específicos de §§3.4/3.6 antes de bajar: un selector diferido conserva methods y un operando inválido conserva NX-E0002. Si una construcción aceptada requiere una capacidad de la tabla anterior, rechazarla antes de bajar con implementation_limit y su pending_scope; no inventar uno por el nombre de esta guarda. Si todas las comprobaciones pasaron, una raíz imposible en IR es InvalidIr controlado, exit 2. |

Las demás ramas de OutsideScalarSlice siguen la misma clasificación: los rechazos de alcance existentes conservan su feature_id (por ejemplo partial_transfer); Result/propagación pendientes usan implementation_limit/I08; las construcciones ya soportadas no se degradan a exclusiones. El inventario de cada rama y su positivo/negativo se comprueba al retirar la variante.

Para la frontera compuesta de A2 §13.13, usar `T="(if true then 1 else 1)"` y `fuente(n)="fn Main(){let x=" + unir n copias de T con "+" + ";}"`, ASCII sin LF final: 127 términos, **3.065 bytes**, altura 128, aceptados; 128 términos, **3.089 bytes**, altura 129, NX-E0003/expression_tree_depth en parse, primario `[16,3087)`. Con átomos `1`, 127 y 128 términos se aceptan y 129 exceden altura 128. Es obligación de I10a, no permiso para conservar una guarda interna más restrictiva durante la migración.

No hay publicación intermedia con corpus a medio migrar. Ante fallo se mantiene el último estado verde y la lista nominal de pendientes; no se declara equivalencia ni se distribuye un frontend mixto. No se distribuye una ruta heredada para cubrir capacidades aún pendientes.

### 7.2. Qué se regenera y qué se reescribe

| Clase | Acción |
|---|---|
| Positivos y negativos semánticos de F0-0002 | Regenerar fuente desde AST; preservar oráculos y reglas; offsets por escaneo independiente |
| Los 15 archivos de corpus heredado y sus consumidores CLI | Reescribir a mano en B; esperados de A2 §13.4, transcritos; ninguno de los 15 requiere implementation_limit (§7.1.6). Las capacidades pendientes se registran solo en nuevas fixtures que las requieran. |
| Plantillas y fuzz de programas válidos | Generador de AST independiente → print B; comparar N y comportamiento |
| Negativos de lexer/parser, FIRST, modos contextuales, `then`, coma final | Reescribir regla de mutación aislada; positivos regenerados; esperados literales independientes |
| Tests del impresor y formato | Sustituir snapshots por B; probar precedencia, comentarios, nombres y las propiedades §5.2 |
| Diagnósticos JSON/texto/explain | Regenerar fuentes; transcribir nuevos tokens, spans, perfil y reparaciones de §6; no borrar campos |
| Datos mock, salidas, eventos y límites | Conservar; nombres de biblioteca no renombran el protocolo de observación |
| Documentos históricos | Conservar íntegros y enumerarlos; documentación vigente debe señalar B y sus programas migrados |

Inventario de archivos y dependencias de la sustitución (los nombres enumerados son exhaustivos para el árbol actual; cualquier sitio añadido antes de implementarla se incorpora al censo del mismo corte):

| Acción | Archivos / partes |
|---|---|
| Borrar físicamente | `src/syntax.rs`, `src/runtime.rs`. |
| Retirar el prototipo dentro de archivos conservados | `src/check.rs`: imports de syntax/Diagnostic, Ty/Binding/Checker heredados y check(&[Stmt]); conservar names/types/check_proposed. `src/lib.rs`: módulos syntax/runtime, Diagnostic/render, CheckedProgram y compile; publicar el pipeline nuevo. |
| Reescribir entrada y validación integrada | `src/main.rs`, `tests/cli.rs`, `tests/corpus.rs`, `README.md`; los tests usan entradas B y contratos correspondientes. Los 10 tests que ejecutan programas en tests/corpus.rs son `positive_corpus`, `nested_copy_output`, `transfer_has_one_owner_and_exact_cleanup`, `early_return_unwinds_scopes_in_reverse_order`, `recoverable_failure_and_partial_initialization`, `explicit_close_and_failure_do_not_double_close`, `indexing_and_overflow_are_checked_in_every_build`, `assignment_bounds_preserve_cleanup`, `assignment_checks_negative_bounds_after_rhs` y `work_budget_exhaustion_closes_resources`. En `work_budget_exhaustion_closes_resources` se reescribe el esperado R0001 a R0002 con `runtime_budget_kind=live_value_cells`, emitido por el pipeline nuevo conforme a A2 §§11.3/13.5. `scripts/validate.sh` conserva fmt, clippy y cargo test en debug/release, con --locked/--offline donde ya figuran; cargo test debe ejecutar las regresiones B y de lectura/uso descritas debajo. |
| Retirar comparador operativo | Borrar `scripts/compare-head.sh` y sus invocaciones operativas: su comparación literal contra el prototipo anterior no puede exigir «0 differences» tras retirarlo. Sus casos de lectura acotada, UTF-8, corpus, uso y archivo inexistente pasan a tests/cli.rs/tests/corpus.rs con esperados normativos B; regla de regresión debajo. |
| Actualizar (no reescribir) | `HANDOFF.md` (incluidos `HANDOFF.md:69`, el párrafo sobre el `move` provisional, y `HANDOFF.md:104`), `Pending.md` (incluidos `Pending.md:16` y `Pending.md:50`), `RECOVERY.md` y `docs/F0_ACUERDOS.md` §9.6. HANDOFF cambia la retirada «en I11» por retirada en el corte B y describe la vía única y sus límites. Pending sustituye el estado vigente de ruta heredada por capacidades pendientes I07c/I08–I11 y conserva las puertas de fase. RECOVERY actualiza los comandos de reproducción a validate.sh y las regresiones B, sin exigir compare-head.sh ni «0 differences». En los tres, las afirmaciones previas de ruta heredada y resultados del comparador se identifican como evidencia histórica de su commit, no como comportamiento ni instrucciones actuales; se preservan los datos útiles. F0_ACUERDOS §9.6 sustituye «candidata a evaluar» por B como superficie oficial, remite a este ADR y conserva criterios y medición posteriores. Estas actualizaciones corresponden al coordinador al integrar el corte. |
| Sustituir superficie, tablas y fuentes embebidas | `src/frontend/lex.rs`, `src/frontend/parse.rs`, `src/frontend/parse/tests.rs`, `src/frontend/print.rs`, `src/frontend/mod.rs`, `src/diagnostic.rs`, `src/check/names.rs`, `src/check/types.rs`, `src/check/tests.rs`, `tests/canonical_print.rs`, `tests/i01_diagnostic.rs`. |
| Integrar pipeline y migrar fixtures del núcleo | `src/ir/lower.rs`, `src/ir/mod.rs`, `src/ir/model.rs`, `src/ir/verify.rs`, `src/ir/ownership_flow.rs`, `src/ir/ownership_flow/sparse.rs`, `src/ir/tests.rs`, `src/ir/i05_tests.rs`, `src/ir/i06_tests.rs`, `src/ir/i07a_tests.rs`, `src/ir/i07b_tests.rs`, `src/interp/mod.rs`, `src/interp/machine.rs`, `src/interp/observation.rs`. Solo conexiones de entrada/origen, tablas y fuentes de pruebas donde proceda; sin alterar reglas semánticas para aceptar B. |
| Reescribir manualmente los cinco positivos | `corpus/pass/integer_limits.nx`, `nested_copy.nx`, `resource_transfer.nx`, `return_cleanup.nx`, `shadow_scope.nx` (todos bajo corpus/pass/). |
| Reescribir manualmente los diez negativos | `corpus/fail/escape_scope.nx`, `immutable_write.nx`, `mixed_array.nx`, `nested_resource.nx`, `overflow_literal.nx`, `resource_copy.nx`, `trailing_garbage.nx`, `unreachable_resource_copy.nx`, `use_after_drop.nx`, `use_after_move.nx` (todos bajo corpus/fail/). |
| Catálogo normativo que se actualiza al implementar | `docs/adr/F0-CODIGOS.md`: las inserciones y sustituciones de §§3.6/6.5, incluidos FEATURE_IDS, pending_scope y explain; no se clasifica como evidencia histórica. |
| Evidencia histórica excluida de ejecución | ADR anteriores salvo las enmiendas normativas expresas de §§6/8, baseline intacta y `docs/validation/`, incluidos los arneses F0-0001; `docs/proposals/sintaxis-candidata-externa-1.md` es documento histórico (fuente EXT de F0-D15; no se reescribe); no se llaman desde CLI, suite nueva o scripts operativos. Se conserva su contenido como historial, no como frontend ni corpus activo. |

Los 15 archivos se mapean individualmente a A2 §13.4 y §§8.7–8.11 de este ADR: se preservan las causas y observaciones soportadas; cualquier nueva fixture dependiente de Result/entrada, efectos, cuotas completas o flujos registra la etiqueta y corte de §7.1. No se marca pendiente una propiedad que ya ofrece el núcleo para evitar probarla. Los nombres de archivo son identidades de fixture, no grafías aceptadas del lenguaje; use_after_move mantiene su identidad y usa `move` de B. No quedan dependencias de nexo::runtime, nexo::syntax o nexo::compile en consumidores activos.

**Regla que reemplaza compare-head.sh.** tests/cli.rs y tests/corpus.rs comparan códigos, campos, spans, salidas, eventos y exit contra los literales de A2 enmendados por §§6/8, en debug y release; conservan también todas las clases de bytes inválidos y fronteras limit/limit+1 del script retirado. La batería independiente sigue contrastando check/run con una copia local de `git archive HEAD`, como exige AGENTS.md, pero registra las diferencias intencionales de este ADR (grafía, catálogo/perfil, presentación, retirada de API y rechazo de contratos no verificados) por caso y campo, sin exigir igualdad byte a byte con el prototipo. No se usa un comodín que ignore todos los diagnósticos ni se autoriza una regresión no enumerada. Las omisiones de métricas de §6.1 se informan con su obligación pendiente y esperado normativo; ningún informe de «0 differences» sustituye esta comparación ni acredita conformidad v2. validate.sh sigue siendo el punto de entrada operativo y no llama a arneses históricos.

Las fixtures que prueban bytes, trivia o cuotas tienen una **transformación posterior declarada** a la impresión canónica, igual que las variantes CRLF y las mutaciones sintácticas: sin introducir otra sintaxis; las fuentes manuales declaradas arriba conservan sus bytes literales. Para las familias de análisis de A2 §11.8, el arnés conserva la secuencia de tokens B impresa y suprime trivia entre tokens, dejando exactamente un espacio ASCII entre dos tokens alfanuméricos adyacentes; si la concatenación cambiara el token por matching más largo, deja también un espacio. Esas familias no contienen comentarios, cadenas ni terminadores implícitos. Se conserva `;`, por lo que quitar saltos no cambia la gramática. Se compara N del AST que se imprime con N del AST de la fuente compacta, sin exigir el reparseo de la impresión canónica que exceda la cota de §5.2, y se cuenta UTF-8 con una rutina independiente. Así la indentación de 65.537 bindings no provoca `source_bytes` antes de `ir_items`. Los tamaños documentales de las fuentes A de §11.8/COD no son esperados de bytes B: se sustituyen por el tamaño del generador B definido aquí, manteniendo las cotas constructivas de cargos, cuotas y aislamiento de la guarda. Un tamaño de la fuente compacta todavía excesivo es fallo de la fixture, no permiso para aumentar la cuota. B-D1, su variante con Later, B-L1, B-G1 y las familias de profundidad/tamaño de §8.13 quedan exentas de la propiedad de reparseo canónico cuando su impresión excede el presupuesto de bytes de fuente. Se comprueban mediante la transformación posterior declarada: fuente compacta publicada → resultado esperado, conservando los diagnósticos, cargos y offsets de esa fuente. Al publicar sus impresiones canónicas se comprueban además su tamaño en bytes UTF-8 y el diagnóstico esperado del reparseo, NX-E0003/source_bytes si exceden la cota; esta comprobación no sustituye la de la fuente compacta. Las pruebas ordinarias y de formato conservan la salida canónica sin este paso y aplican el dominio acotado de §5.2; las de espacio/EOF conservan su transformación literal específica.

### 7.3. Criterios medibles de aceptación de la migración

- 100% de sitios inventariados con destino o excepción histórica explícita; cero fuentes operativas A restantes. Cero parsers/fallbacks de A en el árbol activo y cero frontend heredado pendiente en ese mismo corte; comprobar ausencia física de syntax.rs/runtime.rs y de las API heredadas.
- Para cada fixture analizable por el frontend F0-0002 de referencia, igualdad de N(AST) e identidades bajo mapa declarado; misma IR normalizada y mismo resultado, salida, eventos, efectos, orden de limpieza y trabajo, en debug y release. El corpus heredado manual se contrasta con sus contratos literales, sin exigir un AST de A2 que nunca existió. Cero cambios semánticos exigidos por la traducción; las capacidades pendientes se rechazan explícitamente según §7.1.
- Para cada negativo semántico, mismo código/subcausa/etapa y campos semánticos; primario y relacionados en el nodo previsto. F2-F01 es la única retirada de negativa por adopción; LX01/LX02 son cambios de etapa expresamente enumerados. Las filas de cadenas de §2.4 fijan además el rechazo léxico nuevo de escapes desconocidos. La aceptación sin verificar no_io se retira intencionalmente según §7.1; estos cambios conservan regresiones aisladas.
- El perfil emitido es f0-0002-b-v2 (§6.1). Cada discrepancia conocida de inferencia/procedencia, presupuesto/metadatos o ticks/spans/reservas conserva su esperado de A2 §17.2 y obligación I07c/I10a/I10b. Se informa como fallo pendiente de esa obligación, sin alternar esperados v1, omitir campos o contar implementation_limit como conformidad. La equivalencia de superficie no cierra esas obligaciones.
- Round-trip y formato idempotente en todas las fixtures aceptadas del dominio acotado de §5.2 y en al menos 10.000 AST pequeños generados dentro de ese dominio con semilla registrada; las fixtures de presupuesto exentas de reparseo canónico se comprueban según §§7.2 y 8.13; fuzz válido/inválido sin panics, con presupuestos de A2 respetados y mismos diagnósticos por `check/run` antes de la comprobación específica de entrada.
- Un negativo aislado y un positivo por cada regla textual cambiada, comprobando código, subcausa, token, span y campos exactos. Mutación inversa verificada por regla y restauración SHA-256; esperados transcritos, no calculados por el parser/impresor que se valida. Batería independiente contra archivo local de HEAD, conforme AGENTS.md, sin modificar el checkout de otro agente.
- Cobertura de EX01–EX07 en superficie completa: ocho archivos, mapa de identificadores del 100%, selección/cortocircuito/capturas, futuro y módulos incluidos. En F0 se verifican rechazos de alcance y variantes reducidas declaradas; no se informa ejecución real de EX02/EX03/EX04 o async donde falta biblioteca/runtime. Las puertas siguen EX01/EX04 F2, EX03 F3, EX02 F5 (R §0.3; D14 §§7–8).
- CLI de extremo a extremo sobre B: check acepta biblioteca sin Main y rechaza errores estáticos; run compila, verifica y ejecuta el subconjunto soportado con salida/eventos/exit exactos; las capacidades de §7.1 producen NX-E0006 estable. tests/corpus.rs y tests/cli.rs invocan exclusivamente ese pipeline. Casos literales mínimos: `check` de `fn F(){}` devuelve stdout `nexo-check:ok` seguido de LF, stderr vacío y exit 0 y `run` de la misma biblioteca NX-E0112/missing_main; `run` de `fn Main(){Print(1+2);}` produce exactamente stdout `3` seguido de LF y stderr `nexo-result:()` seguido de LF, sin eventos y con exit 0; hasta implementar I08, `run` del ejemplo Result de §6.1 devuelve NX-E0006/implementation_limit, pending_scope I08, sin salida ni eventos. Los negativos de propiedad ya soportados conservan sus diagnósticos semánticos.
- Búsqueda reproducible con `rg` (equivalente a grep) de `inout`, `consume`, `fn main`, `i64`, `bool`, nombres snake_case del preámbulo y formas if expresión sin then en src/, tests/, corpus/, scripts/, README.md, HANDOFF.md, Pending.md, RECOVERY.md, docs/F0_ACUERDOS.md y docs/adr/F0-CODIGOS.md; añadir los demás archivos del inventario §7.2 y excluir únicamente sus sitios históricos clasificados. Buscar también `compare-head.sh`, `0 differences`, `ruta heredada`, `en I11`, `candidata a evaluar`, `compile`, `nexo::runtime`, `syntax::` y los módulos retirados: cero referencias operativas obsoletas y cero dependencias activas. Los cuatro documentos operativos no se excluyen enteros; cada coincidencia se clasifica por ubicación como instrucción vigente actualizada o evidencia histórica de un commit. La búsqueda distingue gramática fuente, Name ordinarios, identificadores internos estables (`Mode::Consume`, `value.consume`), pruebas **negativas B** de grafías retiradas y citas históricas. Una palabra compartida como `fn` o el identificador interno `inout` no demuestra compatibilidad. Cero coincidencias sin clasificar y cero programas A operativos; los negativos activos usan un programa B con un único defecto, no un parser de A.

Se mantienen visibles las puertas documentales de R F0, incluidos dos revisores sin discrepancias y las decisiones de fase reservadas al propietario (AC §7). La medición posterior no sustituye estas obligaciones.

## 8. Corpus literal de aceptación B

### 8.1. Convenciones del corpus

Los EX de §§8.2–8.6 son entradas válidas, con terminadores por salto, no salida canónica de `fmt`. El impresor usa `;` en toda sentencia simple (§5.1.3) para hacer explícitos sus límites, impedir fusiones al comenzar otra línea por `(`, `[` o `-` y conservarlos al compactar o mover líneas. No cambia el significado de las entradas.

Los programas completos de diseño EX01–EX04 se transcriben de D14 §7, variante B. Sus entradas, resultados y fases se indican con cada uno. No son programas ejecutables F0 por decreto. Las fixtures F2 conservan identidad de A2 §14; `⟦…⟧` marca un primario y se quita antes de contar bytes. Las fuentes de una línea no tienen LF final. Los bloques L tampoco tienen LF tras su última línea. Los programas de ejecución conservan sus observables independientes de A2; no se obtienen ejecutando el compilador bajo prueba.

### 8.2. EX01 — F2

EX01 — F2. Entrada `12`, `8`, `5`: `Total: 25`; líneas vacías ignoradas; error de lectura/parseo/desbordamiento propagado. Biblioteca real y limpieza exigidas (R EX01; D14 §7.2).

Archivo `total.nx`:

```nexo
use std.app
use std.cli
use std.console
use std.fs
use std.parse

fn Main() -> Result<(), app.Error> {
    let path = cli.Positional(0, default: "ventas.txt")?
    let text = fs.ReadText(path)?
    var total: I64 = 0
    for line in text.Lines() {
        let field = line.Trim()
        if field.IsEmpty() {
            continue
        }
        let value = parse.I64(field)?
        total = total.TryAdd(value)?
    }
    console.Println("Total: {total}")?
    return Ok(())
}
```

### 8.3. EX02 — F5

EX02 — F5. `/salud` devuelve JSON `{"estado":"ok"}`; método no permitido 405, ruta no encontrada 404; conserva límites 64/16384/1048576/5000 y cierre automatizado (R EX02; D14 §7.3).

Archivo `salud.nx`:

```nexo
use std.app
use std.http
use std.json

@derive(JsonEncode)
struct Salud {
    estado: String
}

fn Responder(req: view http.RequestView) -> Result<http.Response, app.Error> {
    if req.Method != http.Method.Get {
        return Ok(http.Response.Text(405, "Método no permitido"))
    }
    if req.Path != "/salud" {
        return Ok(http.Response.Text(404, "Ruta no encontrada"))
    }
    let body = json.Encode(Salud(estado: "ok"))?
    return Ok(http.Response.Json(200, move body))
}

fn Main() -> Result<(), app.Error> {
    let server = http.Bind(
        "127.0.0.1:8080",
        maxConnections: 64,
        maxHeaderBytes: 16384,
        maxBodyBytes: 1048576,
        idleTimeoutMs: 5000
    )?
    http.Serve(move server, handler: Responder)?
    return Ok(())
}
```

### 8.4. EX03 — F3

EX03 — F3. Posiciones tras dt=0.5: 0.5 y 1.5; sustituir únicamente Soa por Aos conserva algoritmo y resultado. Unicidad y consola se contabilizan fuera del contrato del núcleo (R EX03; D14 §7.4).

Archivo `particulas.nx`:

```nexo
use std.app
use std.console
use std.table.{Table, TableLayout, UniqueTableView, Soa, Aos}

struct Particula {
    x: F32
    vx: F32
}

fn Avanzar<L: TableLayout>(ps: edit UniqueTableView<Particula, L>, dt: view F32)
    with no_alloc, no_block, no_suspend, no_io {
    for edit p in ps {
        p.x += p.vx * dt
    }
}

fn Main() -> Result<(), app.Error> {
    var ps = Table<Particula, layout: Soa>.FromValues([
        Particula(x: 0.0, vx: 1.0),
        Particula(x: 2.0, vx: -1.0)
    ])
    with ps.PrepareUnique() as view {
        Avanzar(edit view, dt: 0.5)
    }
    console.Println("Primera posición: {ps.Row(0).x}")?
    return Ok(())
}
```

### 8.5. EX04 — F2

EX04 — F2. Salida `0`, `1`, `4`, `9`, una línea por elemento, sin array materializado; abandono/error limpia el estado una vez. Ensanchamiento U32→U64 exacto (R EX04; D14 §7.5).

Archivo `cuadrados.nx`:

```nexo
use std.app
use std.console

// Para un U32, el cuadrado cabe en U64.
gen fn Cuadrados(limite: view U32 = 4) -> U64 {
    for n in 0..<limite {
        let wide = U64(n)
        yield wide * wide
    }
}

fn Main() -> Result<(), app.Error> {
    for value in Cuadrados(limite: 4) {
        console.Println("{value}")?
    }
    return Ok(())
}
```

### 8.6. EX05–EX07: cobertura adicional obligatoria

EX05 conserva Ok(5), contador 0 para false; Ok(8), contador 1 para true; copia de base independiente y cortocircuito observable. EX06: Main construye y destruye futuro inerte sin ejecutar Padre; el arnés futuro espera Ok(7) normal y Err(9) al cancelar hija; no se inventa RunAsync. EX07: Doble(Dato(Valor: 6)) = 12 y Main devuelve Ok(()). Son los contratos de D14 §§8.1–8.3; en F0 conservan exclusión, no ejecución ficticia.

Archivo `capturas.nx`:

```nexo
fn Marcar(cuenta: edit I64) -> Bool {
    cuenta = cuenta + 1
    return true
}

fn Calcular(activo: view Bool, cuenta: edit I64) -> Result<I64, I64> {
    var base: I64 = 3
    let sumar = fn [copy base](x: view I64) -> I64 { return base + x; }
    base = 100
    let value = if activo and Marcar(edit cuenta) and not (base == 0) then {
        let temporal: I64 = 5
        value: temporal
    } else 2
    let resultado: Result<I64, I64> = Ok(sumar(value))
    return match resultado {
        Ok(n) => { value: Ok(n); },
        Err(e) => { value: Err(e); }
    }
}

fn Main() -> Result<(), I64> {
    var cuenta: I64 = 0
    let inactivo = Calcular(false, edit cuenta)?
    if inactivo != 5 {
        return Err(1)
    }
    if cuenta != 0 {
        return Err(2)
    }
    let resultado = Calcular(true, edit cuenta)?
    if resultado != 8 {
        return Err(3)
    }
    if cuenta != 1 {
        return Err(4)
    }
    return Ok(())
}
```

Archivo `tareas.nx`:

```nexo
pub async fn Trabajo() -> Result<I64, I64>
    with {
        create(no_alloc, no_block, no_suspend, no_io),
        resume(no_alloc, no_block, no_suspend, no_io),
        cancel(no_alloc, no_block, no_suspend, no_io),
        drop(no_alloc, no_block, no_suspend, no_io)
    } {
    return Ok(7)
}

async fn Padre() -> Result<I64, I64> {
    region grupo(capacity: 0, executor: local) {
        let h = grupo.Spawn(Trabajo())
        let outcome = await move h
        return match move outcome {
            Completed(move r) => { value: move r; },
            Cancelled => { value: Err(9); }
        }
    }
}

fn Descartar(pendiente: move Future<Result<I64, I64>>) {
    Drop(pendiente)
    return
}

fn Main() -> Result<(), I64> {
    let pendiente = Padre()
    Descartar(move pendiente)
    return Ok(())
}
```

Archivo `main.nx`:

```nexo
mod calculo
use package.calculo.{Dato, Doble}

fn Main() -> Result<(), I64> {
    let dato = Dato(Valor: 6)
    if Doble(dato) != 12 {
        return Err(1)
    }
    return Ok(())
}
```

Archivo `calculo.nx`:

```nexo
pub trait Medida {
    fn Leer(self: view) -> I64 with no_alloc, no_block, no_suspend, no_io
}

pub struct Dato {
    pub Valor: I64
}

impl Medida for Dato {
    fn Leer(self: view) -> I64 with no_alloc, no_block, no_suspend, no_io {
        return self.Valor
    }
}

pub fn Doble<T: Medida>(dato: view T) -> I64
    with no_alloc, no_block, no_suspend, no_io {
    return dato.Leer() * 2
}
```

### 8.7. Fuentes F2 de las tablas de A2 §14

Los primarios se indican por marcadores y por offset literal. Los positivos se obtienen aplicando solo la edición indicada a la fuente sin marcadores. Se conservan las etapas, causas y campos de A2 salvo las enmiendas de §6. Para un positivo de observación, el span identifica la operación, no un error. Ninguna de estas fuentes negativas se somete a fmt como reparación.

**Nombres, tipos, llamadas y operadores**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-S01 | `fn F(x:view I64){} fn Main(){⟦F⟧();}` | `NX-E0111/missing_argument` | `[29,30)` | `F(1);`. |
| F2-S02 | `fn F(x:view I64){} fn Main(){F(1,⟦2⟧);}` | `NX-E0111/excess_positional_argument` | `[33,34)` | Quitar `,2`. |
| F2-S03 | `fn F(x:view I64){} fn Main(){F(⟦y⟧:1);}` | `NX-E0111/unknown_argument_label` | `[31,32)` | Cambiar `y` por `x`. |
| F2-S04 | `fn F(x:view I64){} fn Main(){F(x:1,⟦x⟧:2);}` | `NX-E0111/duplicate_argument` | `[35,36)` | Quitar `,x:2`. |
| F2-S05 | `fn F(x:view I64,y:view I64){} fn Main(){F(x:1,⟦2⟧);}` | `NX-E0111/positional_after_named` | `[46,47)` | Escribir `y:2`. |
| F2-S06 | `fn F(x:edit I64){} fn Main(){var x=1;F(⟦x⟧);}` | `NX-E0111/argument_mode_mismatch` | `[39,40)` | Escribir `edit x`. |
| F2-S07 | `fn Main(){let f=1;⟦f⟧();}` | `NX-E0111/non_callable_target` | `[18,19)` | Quitar `let f=1;`, cambiar la llamada a `F();` y añadir `fn F(){}`. |
| F2-S08 | `fn F(x:edit I64){} fn Main(){let x=1;F(edit ⟦x⟧);}` | `NX-E0105` | `[44,45)` | Cambiar `let x` por `var x`. |
| F2-S09 | `fn Main(){let r=⟦Ok(1)⟧;}` | `NX-E0113` | `[16,21)` | Anotar `r:Result<I64,I64>`. |
| F2-S10 | `fn Main(){let xs=⟦[]⟧;}` | `NX-E0103` | `[17,19)` | Anotar `xs:Array<I64>`. |
| F2-S11 | `fn F()->I64 { ⟦}⟧ fn Main(){}` | `NX-E1317` | `[14,15)` | Insertar `return 1;` antes del cierre marcado. |
| F2-S12 | `fn F()->I64{return ⟦true⟧;} fn Main(){}` | `NX-E0102/type_mismatch` | `[19,23)` | Cambiar el operando por `1`. |
| F2-S13 | `fn F(){} fn ⟦F⟧(){} fn Main(){}` | `NX-E0104`, `declaration_kind = function` | `[12,13)` | Renombrar la segunda función a `G`. |
| F2-S14 | `fn F(x:view I64,⟦x⟧:view I64){} fn Main(){}` | `NX-E0104`, `declaration_kind = parameter` | `[16,17)` | Renombrar el segundo parámetro a `y`. |
| F2-S15 | `fn Main(){⟦Ausente⟧();}` | `NX-E0101`, `use_kind = call_target` | `[10,17)` | Añadir `fn Ausente(){}` al módulo. |
| F2-S16 | `fn Main(){let b=true⟦<⟧false;}` | `NX-E0102/type_mismatch` | `[20,21)` | Sustituir los operandos por `0` y `1`. |
| F2-S17 | `fn Main(){let b=[1]⟦<⟧[2];}` | `NX-E0102/type_mismatch` | `[19,20)` | Sustituir `<` por `==`. |
| F2-S18 | `fn Main(){let b=1==⟦true⟧;}` | `NX-E0102/type_mismatch` | `[19,23)` | Sustituir `true` por `1`. |
| F2-S19 | `fn Main(){let r=MockOpen(1);let b=r⟦==⟧r;}` | `NX-E0102/type_mismatch` | `[35,37)` | Sustituir `let b=r==r;` por `Inspect(r);`. |
| F2-S20 | `fn Main(){let a:Result<I64,I64>=Ok(1);let b=a⟦<⟧a;}` | `NX-E0102/type_mismatch` | `[45,46)` | Sustituir `<` por `==`. |
| F2-S21 | `fn Main(){let a:Result<MockResource,I64>=Ok(MockOpen(1));let b=a⟦==⟧a;}` | `NX-E0102/type_mismatch` | `[64,66)` | Sustituir `let b=a==a;` por `Drop(a);`. |
| F2-S22 | `fn Main(){let ⟦match⟧=1;}` | `NX-E0002` | `[14,19)` | Sustituir el nombre por `matchValue`. |
| F2-S23 | `fn Main(){let x=1;let ⟦x⟧=2;}` | NX-E0104; `data.declaration_kind = binding` | `[22,23)` | Renombrar el segundo binding a `y`. |
| F2-S24 | `fn F(x:view I64){let ⟦x⟧=1;} fn Main(){}` | NX-E0104; `data.declaration_kind = binding`; parámetro original relacionado | `[21,22)` | Encerrar `let x=1;` en un bloque hijo. |
| F2-S25 | `fn Main(){let x=⟦1⟧[0];}` | NX-E0102/not_indexable | `[16,17)` | Sustituir la base marcada por `[1]`. |
| F2-S26 | `fn Main(){let x=1;Inspect(⟦x⟧);}` | NX-E0102/resource_required | `[26,27)` | Sustituir el inicializador de `x` por `MockOpen(1)`. |
| F2-S27 | `fn Main(){let x=1⟦*⟧2;}` | NX-E0002 | `[17,18)` | Sustituir `*` por `+`. |
| F2-S28 | `fn Main(){let x=1⟦!=⟧2;}` | NX-E0002 | `[17,19)` | Sustituir `!=` por `==`. |
| F2-S29 | `fn Main(){let a=[1];Drop(⟦a[0]⟧);}` | NX-E0006, `feature_id = partial_transfer`, `stage = arguments` | `[25,29)` | Sustituir la proyección por `a`; destruye el binding completo. |
| F2-S30 | `fn F(x:edit I64){} fn Main(){F(edit ⟦1⟧);}` | NX-E0002, `stage = parse`, `parser_context = inout_place` | `[36,37)` | Insertar `var x=1;` antes de la llamada y sustituir el literal marcado por `x`. |
| F2-S31 | `fn Main(){let x=move ⟦1⟧;}` | NX-E0002, `stage = parse`, `parser_context = consume_place` | `[21,22)` | Quitar `move`; el literal ya produce un temporal propietario. |
| F2-S32 | `fn Main(){let x=1⟦>⟧2;}` | NX-E0002, `stage = parse`, `unsupported_binary_operator` | `[17,18)` | Sustituir `>` por `<`. |
| F2-S33 | `fn Main(){let x=1⟦<=⟧2;}` | NX-E0002, `stage = parse`, `unsupported_binary_operator` | `[17,19)` | Sustituir `<=` por `<`. |
| F2-S34 | `fn Main(){let x=1⟦>=⟧2;}` | NX-E0002, `stage = parse`, `unsupported_binary_operator` | `[17,19)` | Sustituir `>=` por `<`. |

**Propiedad y préstamos**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-P01 | `fn Main(){let r=MockOpen(1);let s=⟦r⟧;}` | `NX-E0201` | `[34,35)` | `let s=move r;`. |
| F2-P02 | `fn Main(){let r=MockOpen(1);let s=move r;Inspect(⟦r⟧);}` | `NX-E0202/definitely_consumed` | `[49,50)` | Inspeccionar `s`. |
| F2-P03 | `fn Main(){let x=1;Drop(x);Print(⟦x⟧);}` | `NX-E0202/definitely_consumed` | `[32,33)` | Mover `Print(x);` antes de `Drop(x);`. |
| F2-P04 | `fn F(x:view MockResource)->MockResource{return move ⟦x⟧;} fn Main(){}` | `NX-E1105/permission_escalation` | `[52,53)` | Cambiar solo el modo del parámetro a `x:move MockResource`. |
| F2-P05 | `fn F(x:edit I64,y:edit I64){} fn Main(){var x=1;F(edit x,edit ⟦x⟧);}` | `NX-E1105/overlapping_mutable_borrows` | `[62,63)` | Declarar `var y=2;` antes de la llamada y usar `edit y` como segundo argumento. |
| F2-P06 | `fn F(x:view I64,y:edit I64){} fn Main(){var x=1;F(x,edit ⟦x⟧);}` | `NX-E1105/active_borrow_conflict` | `[57,58)` | Primer argumento `x+0`, temporal independiente. |
| F2-P07 | `fn F(x:edit I64,y:edit I64){} fn Main(){var a=[1,2];F(edit a[0],edit ⟦a[1]⟧);}` | `NX-E1105/overlapping_mutable_borrows` | `[69,73)` | Añadir `var b=[3,4];` antes de la llamada y sustituir el lugar marcado por `b[1]`. |
| F2-P08 | `fn F(b:view Bool){let r=MockOpen(1);if b{Drop(r);}Inspect(⟦r⟧);} fn Main(){}` | `NX-E0202/maybe_consumed` | `[58,59)` | Añadir `return;` después de `Drop(r);` en la rama. |
| F2-P09 | `fn F(b:view Bool){let r=MockOpen(1);while b{Drop(⟦r⟧);}} fn Main(){}` | `NX-E0202/maybe_consumed` | `[49,50)` | Mover `let r=MockOpen(1);` al comienzo del cuerpo del bucle. |
| F2-P10 | `fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=⟦r⟧;Drop(s);}` | `NX-E0201` | `[63,64)` | Sustituir el inicializador marcado por `move r`. |
| F2-P11 | `fn Main(){let r:Result<I64,MockResource>=Ok(1);let s=⟦r⟧;}` | `NX-E0201` | `[53,54)` | Sustituir el inicializador marcado por `move r`. |

**Positivos de propiedad**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-PP01 | `fn Leer(r:view MockResource){Inspect(r);Inspect(r);} fn Main(){let r=MockOpen(1);⟦Leer(r)⟧;Inspect(r);}` | Lectura repetida prestada: `open:0:1`, tres `inspect:0:1`, un `close:0:1`. | `[81,88)` | Positivo independiente: conservar fuente y observar el contrato indicado. |
| F2-PP02 | `fn Inc(x:edit I64){x=x+1;} fn Dos(x:edit I64){Inc(edit x);⟦Inc(edit x)⟧;} fn Main(){var x=0;Dos(edit x);AssertEq(x,2);}` | El primer représtamo termina antes del segundo; resultado 2. | `[58,69)` | Positivo independiente: conservar fuente y observar el contrato indicado. |
| F2-PP03 | `fn Copia(a:view Array<I64>)->Array<I64>{return a;} fn Main(){var a=[1];let b=⟦Copia(a)⟧;a[0]=2;AssertEq(b,[1]);}` | El retorno copia datos del parámetro prestado; modificar el original no modifica `b`. | `[77,85)` | Positivo independiente: conservar fuente y observar el contrato indicado. |
| F2-PP04 | `fn Main(){var r=MockOpen(1);Drop(r);⟦r=MockOpen(2)⟧;Inspect(r);}` | Reinicialización completa: `open:0:1`, `close:0:1`, `open:1:2`, `inspect:1:2`, `close:1:2`. | `[36,49)` | Positivo independiente: conservar fuente y observar el contrato indicado. |
| F2-PP05 | `fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=⟦move r⟧;Drop(s);}` | Transferencia del mismo tipo rechazado por copia en P10; una adquisición y un cierre. | `[63,69)` | Positivo independiente: conservar fuente y observar el contrato indicado. |

**Result**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-R01 | `fn Main()->Result<(),I64>{let x=1⟦?⟧;return Ok(());}` | `NX-E0102/type_mismatch` | `[33,34)` | Operando `TryAdd(1,0)`. |
| F2-R02 | `fn Main(){let x=TryAdd(1,2)⟦?⟧;}` | `NX-E0102/type_mismatch` | `[27,28)` | Anotar resultado `Result<(),I64>` y añadir `return Ok(());`. |
| F2-R03 | `fn Main()->Result<(),Bool>{let x=TryAdd(1,2)⟦?⟧;return Ok(());}` | `NX-E0102/type_mismatch` | `[44,45)` | Cambiar el tipo de error a `I64`. |

**Ramas y terminadores**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-B01 | `fn F(b:view Bool)->I64{return ⟦if⟧ b then {value:1;};} fn Main(){}` | `NX-E1302/missing_else` | `[30,32)` | Añadir `else{value:2;}` después de la rama `then`. |
| F2-B02 | `fn F()->I64{return if true then {1;⟦}⟧else{value:2;};} fn Main(){}` | `NX-E1302/missing_branch_value` | `[35,36)` | Cambiar `1;` por `value:1;`. |
| F2-B03 | `fn F(){let n=if true then {value:1;}else{value:⟦true⟧;};} fn Main(){}` | `NX-E1303` | `[47,51)` | Cambiar el valor marcado por `2`. |
| F2-B04 | `fn Main(){⟦value:⟧1;}` | `NX-E1302/invalid_value_target` | `[10,16)` | Sustituir la sentencia por `let value=1;`. |
| F2-B05 | `fn F()->I64{return if true then {⟦value:⟧}else{value:2;};} fn Main(){}` | `NX-E1302/missing_value_operand` | `[33,39)` | Insertar `1;` tras el marcador. |
| F2-B06 | `fn F()->I64{return if true then {⟦value:⟧1;2;}else{value:2;};} fn Main(){}` | `NX-E1302/non_final_branch_value` | `[33,39)` | Quitar `2;` de la primera rama. |
| F2-B07 | `fn F()->I64{return 1 ⟦}⟧ fn Main(){}` | `NX-E0005` | `[21,22)` | Insertar `;` después de `1`. |
| F2-B08 | `fn F()->I64{return if true then {value:1 ⟦}⟧else{value:2;};} fn Main(){}` | `NX-E0005` | `[41,42)` | Insertar `;` después del primer `1`. |
| F2-B09 | `fn F()->I64{return if true then {value:1;}else{value:⟦true⟧;};} fn Main(){}` | `NX-E0102/type_mismatch` | `[53,57)` | Cambiar el valor marcado por `2`. |

**Efectos**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-E01 | `fn Main() with no_alloc{let a=⟦[1]⟧;}` | `NX-E0412`, conocida, construcción | `[30,33)` | Cambiar el array por un escalar. |
| F2-E02 | `fn Copia(a:view Array<I64>)->Array<I64> with no_alloc{return ⟦a⟧;} fn Main(){}` | `NX-E0412`, conocida, copia | `[61,62)` | Cambiar a resultado `I64` y `return Len(a);`. |
| F2-E03 | `fn Main() with no_block{let ⟦r⟧=MockBlockingOpen(1);}` | `NX-E0413`, conocida, limpieza | `[28,29)` | Usar `MockOpen(1)`. |
| F2-E04 | `fn Hoja(){Print(1);} fn Medio(){Hoja();} fn Main() with no_io{⟦Medio⟧();}` | `NX-E0415`, cadena `Main → Medio → Hoja → Print` | `[62,67)` | Quitar `Print(1);` de `Hoja`. |
| F2-E05 | `fn Main() with no_suspend{⟦MockUnknown⟧();}` | `NX-E0414`, desconocida | `[26,37)` | Sustituir la llamada por `AssertEq(1,1);`. |
| F2-E06 | `fn Main() with no_io{if false{⟦Print⟧(1);}}` | `NX-E0415`, rama conservada | `[30,35)` | Sustituir `Print(1);` por `AssertEq(1,1);`. |

**Contratos**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-W01 | `fn Main() with {call(no_io),⟦call⟧(no_alloc)}{}` | `NX-E0417/duplicate_operation` | `[28,32)` | Quitar `,call(no_alloc)`. |
| F2-W02 | `fn Main() with {⟦resume⟧(no_io)}{}` | `NX-E0417/invalid_operation` | `[16,22)` | Cambiar `resume` por `call`. |
| F2-W03 | `fn Main() with no_io,⟦no_io⟧{}` | `NX-E0417/duplicate_restriction` | `[21,26)` | Quitar la segunda restricción y su coma. |
| F2-W04 | `fn Main() with ⟦{}⟧{}` | `NX-E0417/empty_operation_list` | `[15,17)` | Sustituir el mapa por `{call(unrestricted)}`. |
| F2-W05 | `fn Main() with ⟦unrestricted,no_io⟧{}` | `NX-E0417/unrestricted_mixed` | `[15,33)` | Quitar `unrestricted,`. |
| F2-W06 | `fn Main() with {call(⟦no_io,unrestricted⟧)}{}` | `NX-E0417/unrestricted_mixed` | `[21,39)` | Quitar `,unrestricted`. |
| F2-W07 | `fn Main() with unrestricted,⟦unrestricted⟧,no_io{}` | `NX-E0417/duplicate_restriction` | `[28,40)` | Sustituir la lista completa por `no_io`. |

**Alcance**

| ID | Fuente B literal | Esperado | Span | Positivo / contraste |
|---|---|---|---|---|
| F2-F01 | `fn Main(){let r=MockOpen(1);let s=⟦move⟧ r;}` | Positivo: transferencia sin diagnóstico; una adquisición y un cierre | `[34,38)` | Quitar `move`: NX-E0201 sobre `r`; uso posterior de `r`: NX-E0202/definitely_consumed. |
| F2-F02 | `fn ⟦F⟧(){return 1;} fn Main(){}` | `inferred_non_unit_result` | `[3,4)` | Añadir `->I64` después de `F()`. |
| F2-F03 | `fn Main(){let x=⟦0.5⟧;}` | `float_literal` | `[16,19)` | Sustituir el literal por `0`. |
| F2-F04 | `fn Main(){let n=1;let x=0⟦..<⟧n;}` | `range` | `[25,28)` | Sustituir `..<` por `+`. |
| F2-F05 | `fn Main(){let x=(1⟦,⟧2);}` | `tuple` | `[18,19)` | Quitar `,2`; queda agrupación `(1)`. |
| F2-F06 | `⟦@⟧derive(JsonEncode) fn Main(){}` | `attributes` | `[0,1)` | Quitar `@derive(JsonEncode)`. |
| F2-F07 | `fn F(x:view I64⟦=⟧1){} fn Main(){F(1);}` | `default_argument` | `[15,16)` | Quitar `=1`. |
| F2-F08 | `fn Main(){let f=⟦fn⟧[](){return ();};}` | `closure` | `[16,18)` | Sustituir el inicializador completo por `()`. |
| F2-F09 | `fn F(){} fn Main(){let g=⟦F⟧;}` | `function_value` | `[25,26)` | Sustituir `let g=F;` por `F();`. |
| F2-F10 | `fn F(){} fn Main(){⟦move⟧ F();}` | `receiver_mode` | `[19,23)` | Quitar el prefijo `move`. |
| F2-F11 | `fn Main(){let a=[1];let x=move ⟦a[0]⟧;}` | `partial_transfer` | `[31,35)` | Quitar `move`; se copia el elemento. |
| F2-F12 | `fn Main(){let a=⟦[MockOpen(1)]⟧;}` | `resource_array` | `[16,29)` | Quitar los corchetes del inicializador. |
| F2-F13 | `fn Main() with ⟦no_heap_alloc⟧{}` | `no_heap_alloc` | `[15,28)` | Sustituir por `no_alloc`. |
| F2-F14 | `fn Main(){⟦match⟧ true{true=>{},false=>{}}}` | `match` | `[10,15)` | Sustituir el `match` completo por `if true{}else{}`. |
| F2-F15 | `fn F(x:view ⟦Option⟧<I64>){} fn Main(){}` | `option` | `[12,18)` | Sustituir `Option<I64>` por `Result<I64,I64>`. |
| F2-F16 | `⟦struct⟧ S{} fn Main(){}` | `user_types` | `[0,6)` | Quitar `struct S{}`. |
| F2-F17 | `⟦trait⟧ T{} fn Main(){}` | `traits_generics` | `[0,5)` | Quitar `trait T{}`. |
| F2-F18 | `⟦use⟧ std.app;fn Main(){}` | `modules_visibility` | `[0,3)` | Quitar `use std.app;`. |
| F2-F19 | `⟦extend⟧ I64{} fn Main(){}` | `extensions` | `[0,6)` | Quitar `extend I64{}`. |
| F2-F20 | `fn F(x:view I64)->I64 ⟦from⟧(x){return x;} fn Main(){}` | `provenance_clauses` | `[22,26)` | Quitar `from(x)`. |
| F2-F21 | `⟦const⟧ x:I64=comptime 1;fn Main(){}` | `comptime` | `[0,5)` | Quitar la declaración de constante completa. |
| F2-F22 | `fn Main(){let x=⟦"hola"⟧;}` | `text` | `[16,22)` | Sustituir la cadena por `1`. |
| F2-F23 | `fn Main(){let a=[1];a⟦.⟧Foo();}` | `methods` | `[21,22)` | Sustituir `a.Foo()` por `Len(a)`. |
| F2-F24 | `fn Main(){var x=1;x⟦+=⟧1;}` | `compound_assignment` | `[19,21)` | Sustituir `x+=1;` por `x=x+1;`. |
| F2-F25 | `fn Main(){⟦for⟧ n in [1]{}}` | `for_iteration` | `[10,13)` | Sustituir el bucle completo por `while false{}`. |
| F2-F26 | `fn Main(){while false{⟦break⟧;}}` | `loop_control` | `[22,27)` | Quitar `break;`. |
| F2-F27 | `⟦gen⟧ fn G(){} fn Main(){}` | `generator` | `[0,3)` | Quitar `gen`. |
| F2-F28 | `⟦async⟧ fn F(){} fn Main(){}` | `async` | `[0,5)` | Quitar `async`. |
| F2-F29 | `fn F(x:view ⟦U32⟧){} fn Main(){}` | `unsupported_type` | `[12,15)` | Sustituir `U32` por `I64`. |
| F2-F30 | `fn F(x:view ⟦String⟧){} fn Main(){}` | `unsupported_type` | `[12,18)` | Sustituir `String` por `Array<I64>`. |
| F2-F31 | `fn F(x:view ⟦app.Error⟧){} fn Main(){}` | `unsupported_type` | `[12,21)` | Sustituir la ruta completa por `I64`. |
| F2-F32 | `fn Main(){⟦region⟧ r(capacity:0){}}` | `region` | `[10,16)` | Quitar `region r(capacity:0)`; queda un bloque ordinario. |
| F2-F33 | `fn Main(){let x=⟦self⟧;}` | `methods` | `[16,20)` | Sustituir `self` por `0`. |

F2-E05 se repite cambiando solo `no_suspend` por `no_alloc`, `no_block` y `no_io`: esperados NX-E0412, NX-E0413 y NX-E0415, respectivamente, con evidencia desconocida. F2-F02/F12/F15/F29/F30/F31 son etapa types; F2-F09 es names; los demás F son parse salvo F2-F01 positivo. Los feature_id de la tabla F implican NX-E0006, subcause null, no un sufijo de código. F2-F11 conserva context sobre move.

Las tres variantes de E05, sin LF final, se materializan sin dejar implícita su fuente:

| ID de variante | Fuente B literal | Esperado | Primario |
|---|---|---|---|
| F2-E05/no_alloc | `fn Main() with no_alloc{⟦MockUnknown⟧();}` | NX-E0412, evidencia desconocida | `[24,35)` |
| F2-E05/no_block | `fn Main() with no_block{⟦MockUnknown⟧();}` | NX-E0413, evidencia desconocida | `[24,35)` |
| F2-E05/no_io | `fn Main() with no_io{⟦MockUnknown⟧();}` | NX-E0415, evidencia desconocida | `[21,32)` |

F2-LX01…LX05, F2-K01/K02 y los truncamientos están publicados literalmente en §6.3. Sus positivos sustituyen el único token defectuoso, salvo K02 que sustituye `;` por `{}`. Las variantes LF antes de operadores F2-S32…S34 también se rigen por §6.3.

### 8.8. Positivos y entradas adicionales de A2 §14.1/§14.7/§14.9

Igualdad estructural (A2 §14.1): resultado unidad, sin salida ni eventos; no copiar operandos para comparar.

```nexo
fn Main(){
    let a:Result<Array<I64>,I64>=Ok([1,2]);
    let b:Result<Array<I64>,I64>=Ok([1,2]);
    let c:Result<Array<I64>,I64>=Err(7);
    AssertEq(a==b,true);
    AssertEq(a==c,false);
    AssertEq([1]==[1,2],false);
    AssertEq(()==(),true);
}
```

Entrada ausente: `fn F(){}` sin LF final da, únicamente en `run`, NX-E0112/missing_main, primario EOF `[8,8)`, `data={"entry":"Main"}`. Positivo: añadir `fn Main(){}`. `check` de la biblioteca sin entrada es válido.

Entrada con firma inválida: `fn ⟦Main⟧(x:view I64){}` da NX-E0112/invalid_main_signature, nombre `[3,7)`, firma relacionada `[0,19)`, `data={"entry":"Main"}`; positivo quitar `x:view I64`. El verificador de entrada no ejecuta el programa.

F2-PP06, sombreado del preámbulo: retorna 3, sin E/S ni eventos, contrato no_io conservado. El nombre resuelve la función de usuario.

```nexo
fn Print(x:view I64)->I64{return x+1;}
fn Main() with no_io{
    let n=⟦Print(2)⟧;
    AssertEq(n,3);
}
```

Tres positivos adicionales de A2 §14.9, independientes, resultado unidad y sin eventos:

```nexo
fn Main(){Drop([1][0]);}
```

```nexo
fn Drop(x:view I64){}
fn Main(){let a=[1];Drop(a[0]);AssertEq(a[0],1);}
```

```nexo
fn Main(){let r:Result<I64,I64>=Ok(1);AssertEq(r,r);}
```

El primero destruye temporal; el segundo llama a función de usuario y no consume el array; el tercero exige separación contextual de `>=` al cerrar tipo, sin aceptar el operador de comparación en F0.

### 8.9. Ejecución y limpieza de A2 §14.5

F2-14.5-1, fallo preparando argumentos. Esperado R0101 sobre suma completa; eventos `open:0:1`, `close:0:1`; no se entra al cuerpo de F.

```nexo
fn F(r:move MockResource,n:view I64){}
fn Main(){F(MockOpen(1),⟦9223372036854775807+1⟧);}
```

F2-14.5-2, propagación entre frames. Resultado Err(9), salida de proceso 1, sin salida de programa ni R; eventos `open:0:1`, `open:1:2`, `close:1:2`, `close:0:1`.

```nexo
fn Fallo()->Result<I64,I64>{
    let r=MockOpen(2);
    return Err(9);
}
fn Main()->Result<(),I64>{
    let r=MockOpen(1);
    let n=Fallo()?;
    Print(n);
    return Ok(());
}
```

F2-14.5-3, recurso dentro de Result. Resultado Ok(()); eventos `open:0:3`, `inspect:0:3`, `close:0:3`; el payload no se destruye al retornar de Crear.

```nexo
fn Crear()->Result<MockResource,I64>{
    return Ok(MockOpen(3));
}
fn Main()->Result<(),I64>{
    let r=Crear()?;
    Inspect(r);
    return Ok(());
}
```

F2-14.5-4, reasignación. Eventos `open:0:1`, `open:1:2`, `close:0:1`, `close:1:2`, resultado unidad.

```nexo
fn Main(){
    var r=MockOpen(1);
    r=MockOpen(2);
}
```

F2-14.5-5, prioridad de fallo. R0101 sobre suma completa antes de R0102 por índice; `open:0:1`, `close:0:1` y sin escritura del array.

```nexo
fn Main(){
    let r=MockOpen(1);
    var a=[0];
    a[-1]=⟦9223372036854775807+1⟧;
}
```

F2-14.5-6, argumentos nombrados: salida `["2","1"]`, sin eventos; asociación no cambia evaluación.

```nexo
fn Marca(n:view I64)->I64{Print(n);return n;}
fn F(a:view I64,b:view I64){}
fn Main(){F(b:Marca(2),a:Marca(1));}
```

F2-14.5-7 conserva las recetas de cuotas de A2 §§11.3/11.5, regeneradas desde AST B: R0001 para trabajo, R0002 con `call_frames`, `live_value_cells`, `observation_bytes` para las estructurales. Mismos límites, cargo propuesto y limpieza; no confundir copia de array con agotamiento de trabajo. F2-14.5-8 conserva exactamente el JSON de entrada/observación de A2 §12.4: 27 bytes virtuales en el ejemplo exitoso, error de escritura con su evento y salida 1, agotamiento de colas, lectura vacía, registro intermedio inválido y respuestas fallidas. Se migran los programas de ese apartado mediante el impresor B, no los datos del entorno. Estas dos filas no publicaban un programa nuevo en §14.5.

F2-RT01, fuente sin LF final:

```nexo
fn Main(){Print(⟦-9223372036854775808-1⟧);}
```

R0103, namespace F0-0002-sim, stage runtime, operation value.sub, primario `[16,38)`, data `{}`, sin salida/eventos y exit 1. Positivo cambia solo el sustraendo final a 0: imprime `-9223372036854775808`, unidad y exit 0.

### 8.10. LF, CRLF, CR, contextualidad y EOF: F2-L01…L07

F2-L01:

```nexo
fn F()->I64{
⟦return⟧
(1);
}
fn Main(){}
```

NX-E0102/type_mismatch, types. Positivo: reemplazar `return` + LF + `(1);` por `return (` + LF + `1);`. No se absorbe el operando de la línea siguiente en el negativo.

F2-L02:

```nexo
fn F()->I64{
return if true then {
⟦value:⟧
(1);
}else{value:2;};
}
fn Main(){}
```

NX-E1302/missing_value_operand, parse; precede al fallo derivado de entrega no final. Positivo: reemplazar `value:` + LF + `(1);` por `value: (` + LF + `1);`.

F2-L03:

```nexo
fn value(x:view I64)->I64{return x;}
fn Main(){let x=7;let y=⟦value
(x)⟧;AssertEq(y,7);}
```

Positivo, unidad, sin salida/eventos. Se conserva deliberadamente la función `value` para aislar su condición de Name; `fmt --check` señala su estilo y `check` la acepta. La contraparte de estilo canónico renombra declaración y referencia a `Value` y verifica la misma observación; no reemplaza esta prueba léxica.

F2-L04:

```nexo
fn Main(){
let value=2
let a=⟦value -1⟧
let b=value - 1
AssertEq(a,b)
}
```

Positivo, a=b=1, unidad. Reemplazar `value -1` por `value - 1` conserva N y trabajo. Se conserva la misma función de los saltos insertados.

F2-L05:

```nexo
fn Main(){
let x=if true then {⟦value: ();⟧}else{value: ();};
AssertEq(x,());
}
```

Positivo: entrega explícita de unidad, no marcador vacío. El impresor puede abreviar ambas ramas porque no tienen comentario.

F2-L06:

```nexo
fn F()->I64{
1
⟦}⟧
fn Main(){}
```

NX-E1317, ownership. Positivo: sustituir la línea `1` por `return 1`. El salto del negativo termina un descarte, no un retorno.

F2-L07:

```nexo
// á🙂
fn Main(){
let x=⟦missing⟧;
}
```

NX-E0101, names, `data.use_kind=value`, línea 3, columna 7. Variantes: LF sin salto final, CRLF sin salto final, CR sin salto final, LF con un salto final. El positivo inserta `let missing=1;` inmediatamente antes de `let x=missing;`. Cada variante se materializa en bytes antes de calcular spans.

### 8.11. Objetos y offsets complementarios transcritos

Los offsets de esta sección se calculan contando bytes de las fuentes literales publicadas; no usan el frontend bajo prueba. Son obligatorios junto a las reglas de A2 §§13.6/13.15. No se normalizan CRLF antes de verificarlos.

| Fuente | Span literal |
|---|---|
| F2-L01 | `[13,19)` |
| F2-L02 | `[35,41)` |
| F2-L03 | `[61,70)` |
| F2-L04 | `[29,37)` |
| F2-L05 | `[31,41)` |
| F2-L06 | `[15,16)` |
| F2-L07 | `[27,34)` |
| F2-L07 CRLF | `[29,36)` |
| F2-L07 CR | `[27,34)` |
| F2-L07 LF con LF final | `[27,34)` |
| F2-PP06 | `[71,79)` |
| F2-14.5-1 | `[63,84)` |
| F2-14.5-5 | `[59,80)` |

F2-L01 relacionados: `delimiter=[19,20)`, `expected_type=[8,11)`. F2-L02 relacionados: `delimiter=[41,42)`, `context=[33,34)`. En CRLF el delimiter ocupa dos bytes. F2-L04 con espacio adicional tiene primario `[29,38)`. F2-L06 relaciona el resultado escrito `[8,11)`.

Datos exactos de NX-E1105; todas las claves no publicadas siguen ausentes:

| Fuente | `data` literal B |
|---|---|
| F2-P04 | `{"origin":{"kind":"parameter","id":"parameter:5"},"parameter":{"kind":"parameter","id":"parameter:5"},"active_permission":"read","requested_access":"consume"}` |
| F2-P05 | `{"origin":{"kind":"binding","id":"binding:44"},"binding":{"kind":"binding","id":"binding:44"},"loan":{"kind":"loan","id":"loan:50:0"},"active_permission":"inout","requested_access":"borrow_inout"}` |
| F2-P06 | `{"origin":{"kind":"binding","id":"binding:44"},"binding":{"kind":"binding","id":"binding:44"},"loan":{"kind":"loan","id":"loan:50:0"},"active_permission":"read","requested_access":"borrow_inout"}` |
| F2-P07 | `{"origin":{"kind":"binding","id":"binding:44"},"binding":{"kind":"binding","id":"binding:44"},"loan":{"kind":"loan","id":"loan:54:0"},"active_permission":"inout","requested_access":"borrow_inout"}` |

F2-E04 conserva exactamente: primario `[62,67)`, contrato `[56,61)`; cadena ordenada `function.call` `[62,67)`→`function:24`, `function.call` `[32,36)`→`function:3`, `intrinsic.call` `[10,15)`→`prelude:print`, evidence verified en los tres pasos. Los nombres fuente son Main/Medio/Hoja/Print.

F2-S29: context sobre `Drop`, `[20,24)`, primary `[25,29)`, stage arguments. F2-F11: context sobre `move`, [26,30).

Reparaciones exactas adicionales (status proposed, validation null, expected_text vacío, replacement `;`):

| Fuente | Primario `}` | Prefijo de sentencia | preceding_expression | Inserción |
|---|---|---|---|---|
| F2-B07 | `[21,22)` | `[12,21)` | `[19,20)` | `[20,20)` |
| F2-B08 | `[41,42)` | `[33,41)` | `[39,40)` | `[40,40)` |

F2-B07/B08 conservan expected_tokens `[";"]`, canonical_insertion y statement_terminator. La inserción precede al espacio que separa el operando de la llave. Ningún otro error de las tablas genera una Repair ordinaria.


### 8.12. Inferencia y ubicaciones de la enmienda de A2

T01–T07 corresponden uno a uno a TD01–TD07 de A2 §14.10; D01–D08 a DG01–DG08 de A2 §14.11. Se conservan también las identidades F2-TD01…TD07/F2-DG01…DG08 para trazabilidad. Fuentes ASCII completas sin LF final; los offsets siguientes son transcripciones sobre esos bytes B, independientes del impresor. No se comparan contra la longitud de la fuente A. Tipos serializados I64/Bool sustituyen i64/bool; modos, reglas, prioridades y etapas no cambian.

Para T01–T03: exactamente una NX-E0102/type_mismatch, stage types, operation types.check, outcome rejected, exit 1; cause_chain y repair [], feature_id null, campos sintácticos nulos/listas vacías, rule/explanation_id NX-E0102/type_mismatch. related_spans se ordena por inicio fuente y lleva exactamente expected_type y found_type. No hay ejecución ni observaciones.

| ID | Fuente B literal | Primario y relaciones / esperado |
|---|---|---|
| T01 | `fn Main(){var v=(if true then {return;}else{return;});var w=v;v=3;w=true;}` | Primario/found_type `[68,72)`; expected_type `[64,65)`; data `{"expected_type":"I64","found_type":"Bool"}`. |
| T02 | `fn Main(){var v=3;var w=v;v=3;w=true;}` | Primario/found_type `[32,36)`; expected_type `[16,17)`; mismo data de T01. |
| T03 | `fn Main(){var v=(if true then {return;}else{return;});var w=true;w=v;v=3;}` | Primario/found_type `[71,72)`; expected_type `[60,64)`; data `{"expected_type":"Bool","found_type":"I64"}`. |
| T04 | `fn Main(){let x=((if true then {return;}else{return;}))==true;}` y `fn Main(){let x=true==(if true then {return;}else{return;});}` | Ambos aceptados, diagnostics []; resultado estático de igualdad Bool, no Never. Ejecución unidad sin observaciones. |
| T05 | `fn Main(){let x=[(if true then {return;}else{return;}),true];}` y `fn Main(){let x=[true,(if true then {return;}else{return;})];}` | Ambos aceptados, diagnostics []; elemento Bool, sin I64 provisional. Ejecución unidad sin observaciones. |
| T06 | `fn Main(){var v=(if true then {return;}else{return;});var w=v;v=[1];w=[2];w[0]=9;}` | Aceptado; restricciones Array<I64> compartidas entre v/w, datos independientes. |
| T07 | `fn Main(){let x=missing;let y=x;let z=true+1;}` | Solo NX-E0101, stage names, operation names.resolve, data `{"use_kind":"value"}`, primario `[16,23)`; sin error derivado por y ni comprobación de types después de la barrera. |

Positivos de T01/T02: sustituir solo el último true por 4; T03: sustituir el último 3 por false. Se aceptan con orden inverso de restricciones, otro representante y debug/release. El gemelo vivo literal de T06 es `fn Main(){var v=[0];var w=v;v=[1];w=[2];w[0]=9;AssertEq(v,[1]);AssertEq(w,[9]);}`: unidad, sin observaciones; verifica copia independiente.

En T07, sustituir missing por 0 da exactamente `fn Main(){let x=0;let y=x;let z=true+1;}`: NX-E0102/type_mismatch, primario/found_type `[32,36)`, data `{"expected_type":"I64","found_type":"Bool"}`, sin expected_type fuente por dominio intrínseco. Sustituir además true por 0 da el positivo.

Las variantes aisladas de T04/T05 y contaminación de T07 tienen NX-E0102/type_mismatch, stage types, operation types.check, data `{"expected_type":"I64","found_type":"Bool"}`, found_type sobre su primario, cause_chain/repair [], sin diagnósticos sobre el valor contaminado:

| Variante | Fuente B literal | Primario(s) y expected_type |
|---|---|---|
| T04 negativo izquierdo | `fn Main(){let x=(if true then {return;}else{return;})+true;}` | `[54,58)`; sin expected_type fuente. |
| T04 negativo derecho | `fn Main(){let x=true+(if true then {return;}else{return;});}` | `[16,20)`; sin expected_type fuente. |
| T05 negativo izquierdo | `fn Main(){let x=[(if true then {return;}else{return;}),1,true];}` | `[57,61)`; expected_type `[55,56)`. |
| T05 negativo derecho | `fn Main(){let x=[1,(if true then {return;}else{return;}),true];}` | `[57,61)`; expected_type `[17,18)`. |
| T07 contaminación | `fn Main(){let x=true+1;let y=x;let z=false+1;}` | Dos errores ordenados: `[16,20)` y `[37,42)`; sin expected_type fuente ni error sobre y. |

Los positivos de T04/T05 negativos sustituyen solo el true incompatible por 1; la condición del if no cambia. T07 contaminación sustituye ambos booleanos por 0; reparar uno conserva únicamente el otro error. Never no oculta ninguna contradicción.

D01–D03: R0102, subcause null, namespace F0-0002-sim, stage runtime, operation value.index, data {}, cause_chain/repair [], outcome simulator_fault, exit 1, sin observaciones; única relación context. Su positivo cambia el índice 2 por 0: D01 imprime 0 y D02/D03 no producen salida.

| ID | Fuente B literal | Primario | context |
|---|---|---|---|
| D01 | `fn Main(){let a=[[0]];Print(a[0][2]);}` | `[32,35)` | `[28,35)` |
| D02 | `fn Main(){var a=[[0]];a[0][2]=5;}` | `[26,29)` | `[22,29)` |
| D03 | `fn F(x:edit I64){}fn Main(){var a=[[0]];F(edit a[0][2]);}` | `[51,54)` | `[42,54)`, argumento completo con edit |

La variante con primer índice 2 y segundo 0 usa respectivamente primarios `[29,32)`, `[23,26)`, `[48,51)`; los contextos no cambian. Para D03 etiquetado, `fn F(x:edit I64){}fn Main(){var a=[[0]];F(x:edit a[0][2]);}`, primario `[53,56)`, context `[42,56)` incluye `x:` y `edit`; no comienza en la raíz ni en el prefijo de modo aislado.

D04–D06 son ownership, outcome rejected, exit 1; D07/D08 son types con el mismo resultado de rechazo. Todos tienen data {} salvo el par de tipos declarado, cadenas y relaciones exclusivamente como se indican, repair [], campos sintácticos nulos/listas vacías, feature_id null. Regla/explanation_id es código/subcausa cuando existe, o código solo.

| ID | Fuente B literal | Esperado literal |
|---|---|---|
| D04 | `fn Main(){let a=[0];Drop(a);a[0]=5;}` | Una NX-E0202/definitely_consumed, ownership.use, primario `[28,29)`; declaration `[14,15)`, consumption `[20,27)`; cadena única drop/value.drop, span `[20,27)`, binding:14, evidence verified. NX-E0105 sería derivado. |
| D05 | `fn Main(){let a=[0];Drop(a);a=[1];}` | Una NX-E0105, subcause null, place.write, primario `[28,29)`; declaration `[14,15)`; cadena []. La sustitución completa no lee el valor destruido. |
| D06 | `fn Main(){let a=[0];let b=0;Drop(a);Print(a);b=1;}` | Dos errores ordenados: NX-E0202/definitely_consumed, ownership.use, `[42,43)`, declaration `[14,15)`, consumption `[28,35)`, cadena única drop/value.drop, span `[28,35)`, binding:14, evidence verified; después NX-E0105, subcause null, place.write, `[45,46)`, declaration `[24,25)`, cadena []. |
| D07 | `fn Main(){var a:Array<I64>=[0];a[0]=true;}` | NX-E0102/type_mismatch, types.check, primario/found_type `[36,40)`; expected_type `[16,26)` sobre Array<I64>; data `{"expected_type":"I64","found_type":"Bool"}`; cadena []. |
| D08 | `fn Main(){var a=[0];a[0]=true;}` | NX-E0102/type_mismatch, types.check, primario/found_type `[25,29)`; expected_type `[16,19)` sobre [0]; mismo data de D07; cadena []. |

En las cadenas, binding:14 es Entity `{"kind":"binding","id":"binding:14"}`. Positivos: D04 cambia let por var y quita Drop(a); D05 cambia let por var; D06 mueve Print(a) antes de Drop(a) y declara var b; D07/D08 cambian true por 1. **DG04, DG07 y DG08 de A2 divergen hoy del código**, y sus contrapartes D04/D07/D08 conservan esa obligación: no se acepta NX-E0105 para el acceso indexado indisponible, ni expected_type sobre los destinos `[31,35)`/`[20,24)`. A2 §17.2 asigna D04–D06 a corrección de I07b y D07/D08 a I07c; D01–D03 se cierran en I10b.

### 8.13. B-D1 y continuación con Later en superficie oficial

B-D1, su variante con Later, B-L1, B-G1 y las familias de profundidad/tamaño de esta sección y A2 §11.8 están exentas de la propiedad de reparseo canónico de §5.2 cuando su impresión excede el presupuesto de bytes de fuente de A2 §13.13. En B-D1, incluida la variante con Later, la impresión canónica excede el presupuesto por la sangría de los bindings dentro de los bucles profundamente anidados; su reparseo da NX-E0003/source_bytes. Se aplica la transformación posterior declarada y las comprobaciones de tamaño y diagnóstico de §7.2; los resultados y offsets siguientes corresponden a la fuente compacta publicada.

La expansión B compacta literal, ASCII sin LF final, sustituye la grafía de A2 §11.8.B:

```text
"fn Big(b:view Bool){var s=MockOpen(1);Drop(s);while b{Inspect(s);s=MockOpen(2);}"
+ "while b{" repetido 96 veces
+ para j=0..5999: "let x"+j+"=0;"
+ "}" repetido 97 veces
+ "fn Main(){}"
```

**71.846 bytes**; Big ocupa `[0,71835)`. Con `fn Later(){let z=0;Drop(z);Print(z);}` antes de Main: **71.883 bytes**. Son tres bytes más que los **71.843/71.880** de A2 §11.8.B; no cambia la IR normalizada: **61.400** ítems de Big + **7** de Main = **61.407** completos, más **14** parciales de Later = **61.421**. Se conserva el rechazo local del cargo **1.000.001**, las **1.768.894** unidades del recorrido completo de Big y las **2** de Main, total **1.768.896**; cobrar Main sin bucles sigue siendo obligación de I10a (A2 §11.1). La equivalencia de B debe comprobarse al implementar el parser, no se afirma haber ejecutado B con el código actual.

Para B-G1, la familia N de A2 §11.8.B usa en B `fn F0(b:view Bool){`, los mismos 96 bucles y 1.024 bindings, y cierre con 97 llaves; se repite para F0…F13 y se añade `fn Main(){}`. Son **168.883 bytes**, frente a **168.813** de A2 (cinco bytes adicionales por cada uno de los 14 modos view). Conserva **162.309 ítems**, **305.906 unidades por N**, **2** de Main y **4.282.686 unidades** completas; rechazo global en **4.000.001**, sin analysis_function. Con el orden fuente normativo, F13 recibe las **23.222** unidades restantes tras **3.976.778** concedidas a F0…F12; el primario traducido de la medición es `[166263,166264)`, referencia no portable bajo A2 §13.13. El nombre F13 y ese primario presuponen orden fuente, no el orden por nombre del lowering actual; el coste completo no depende de esa diferencia de orden. No se analiza Main tras el rechazo global.

| Orden | Esperado literal B |
|---|---|
| 1 | NX-E0202/maybe_consumed, stage ownership, operation ownership.use, primario `[62,63)`, declaration `[24,25)`, consumption `[38,45)`; cadena única kind drop, operation value.drop, span `[38,45)`, Entity `{"kind":"binding","id":"binding:24"}`, evidence verified; data {}. |
| 2 | NX-E0003, subcause null, ownership/analysis.limit, data `{"budget_kind":"ownership_flow_steps","budget_scope":"function","limit":1000000,"observed":1000001,"incomplete_analysis":"ownership","analysis_function":{"kind":"function","id":"function:3"}}`; related_spans/cause_chain []. Primario válido no vacío dentro del sufijo anidado de Big, después de `[62,63)` y antes de su cierre `[71834,71835)`; el nodo correspondiente a la medición A es `[49740,49741)`, referencia medida traducida, no offset portable obligatorio (§6.1/A2 §13.13). |
| 3, solo con Later | NX-E0202/definitely_consumed, ownership/ownership.use, primario `[71868,71869)`, declaration `[71850,71851)`, consumption `[71854,71861)`; cadena única kind drop, operation value.drop, span `[71854,71861)`, Entity `{"kind":"binding","id":"binding:71850"}`, evidence verified; data {}. |

Perfil f0-0002-b-v2, namespace Nexo-proposed-v1, severity error, rejected/exit 1. Campos comunes y vacíos según A2 §11.8.B; la lista se compara en ese orden y la fila de límite no puede usar un span del nombre de Big que la anteponga al primer error. La validez exige además que el primario corresponda al cargo rechazado dentro de analysis_function, no solo que pertenezca a la fuente.

## 9. Riesgos, alternativas descartadas y calidad posterior

### 9.1. Riesgos y controles

| Riesgo | Control exigido | Consecuencia del hallazgo |
|---|---|---|
| Un renombrado textual cambia una identidad o etiqueta | Mapa de símbolos resueltos, colisiones, sombreado Print/Drop, APIs y JSON | Corregir migración y conservar regresión; nunca inferir categoría por caja |
| Prefijo contextual ambiguo | Pares `F(move)`/`F(move x)`, `move(x)`/`(move)(x)`, `edit view`, saltos y operandos ausentes | Corregir regla/diagnóstico B sin añadir sinónimos |
| If compacto oculta retorno o limpieza | AST con Value, ramas mixtas, return/divergencia, comentarios y entrega no final | Rechazar implementación que sintetice Return o pierda limpieza |
| Igualdad de AST «lograda» borrando significado | Normalización cerrada §4.2; reparseo canónico solo en el dominio acotado de §5.2 y fixtures de presupuesto según §§7.2/8.13; comprobar nombres, etiquetas, modos y contratos aparte | No aceptar un comparador que borre diferencias para satisfacer fixtures |
| Checker acoplado a grafía u offsets A | Tablas de preámbulo/origen; tipo después del modo; archivo de referencia independiente | Reparar la frontera; conservar cero cambios de reglas semánticas |
| Fixtures generadas reproducen un bug del impresor | Corpus textual independiente, mutaciones inversas y esperados del ADR | Invalidar evidencia dependiente y añadir regresión |
| B completa se anuncia como implementada en F0 | Rechazos por feature_id y reporte por sintaxis/estática/ejecución | Mantener explícita la capacidad pendiente y su fase |
| Rango inclusivo desborda al calcular sucesor | Comparar extremo, vacío si lo>hi, máximo/mínimo entero | No bajar inclusivo mediante hi+1 |
| Genéricos y comparación cambian por espacios | Regla de adyacencia, listas anidadas, cierre >= y tests de formato | Revisar B y su CST antes de ampliar soporte |
| Convención visual cambia API observable | fmt separado de refactorización; exportaciones/FFI/serialización protegidas | Informar incumplimiento, sin renombrado silencioso |
| Persistencia del frontend A tras migrar | Inventario de rutas CLI/API y retirada física en el mismo corte de B | Cero archivos del prototipo heredado y cero rutas que acepten A al cerrar el corte |
| Una muestra no demuestra facilidad universal | Resultados por tarea/modelo/cohorte, límites, fallos y revisión independiente | Corregir problemas reproducibles y publicar incertidumbre |

### 9.2. Alternativas descartadas

| Alternativa | Motivo |
|---|---|
| A | Descartada por decisión del propietario; su familiaridad o menor coste histórico no reabre la decisión. Se conservan únicamente documentos/evidencia histórica. |
| C | Descartada por la misma decisión; no se mantiene implementación ni corpus ejecutable competidor. |
| EXT sin adaptar | Añade tubería, fails, prop, defer, take/copy general y retornos con flecha que chocan con los contratos adoptados. B incorpora los arreglos explícitos de D14 §5. |
| Dos superficies o un modo de compatibilidad | Multiplican reglas, diagnósticos y documentación y contradicen la única superficie oficial. |
| Una sintaxis exclusiva para agentes | Las herramientas comparten AST e identidad; no necesitan un segundo lenguaje de usuario. |
| Renombrar todo mediante regex o formateo | Cambia símbolos públicos, etiquetas y serialización sin prueba de identidad. |

### 9.3. Prueba de agentes como control de calidad posterior

Se ejecuta **después de adoptar e implementar B**; no condiciona su elección ni puntúa A/C. Se registra antes de medir un manifiesto con hash de este ADR, ejecutables, corpus, interfaces, prompts, semillas, versiones de dos modelos, presupuestos, oráculos independientes y etapas realmente disponibles. No usar servicios de pago/red sin autorización correspondiente. No se simulan resultados de fases no implementadas.

Se conserva de D14 §9.2 el diseño de tareas aplicado únicamente a B: siete familias EX01–EX07, cuatro tareas de generación y ocho negativos aislados por familia, dos modelos y cinco semillas. Son 280 sesiones de generación y 560 de reparación, **840 sesiones B**. Contexto nuevo por sesión, 6000 tokens iniciales, 3000 por revisión, hasta tres propuestas (inicial y dos revisiones), 180 segundos por propuesta. Timeouts, vacíos, supresión de contratos/funciones o constantes que sustituyen el algoritmo son fallos. Se puntúa antes de formato; usar fmt consume el mismo presupuesto. Modelo/semilla no soportados se registran; no se inventan réplicas independientes.

| Métrica | Umbral que activa revisión de B |
|---|---|
| G1, generación correcta al primer intento | Menos de 85% global o 75% en cualquier familia/modelo |
| R2, reparación correcta dentro del presupuesto | Menos de 90% global o 80% en cualquier familia/modelo |
| E, nuevos episodios causales por sesión | Media mayor de 0,50 en generación o 0,25 tras primera edición de reparación |
| S, aceptaciones que violan el contrato | Cualquier caso; revisar 100% de aceptadas, no muestreo |

S requiere dos revisores independientes del implementador y adjudicación de discrepancias antes de puntuar; sin aceptadas es no evaluado, nunca cero. Un defecto del oráculo invalida puntuaciones afectadas y obliga a recomprobarlas. E usa `(sesión, intento, regla, nodo causal)`; dos diagnósticos de la misma reparación mínima son un episodio. Acuerdo entre codificadores ≥90% y F1 de agrupación ≥0,90; si no se alcanza, corregir manual de codificación y repetir, no reducir episodios arbitrariamente. Conservar fuentes, hashes, diagnósticos, ediciones, trazas y veredictos individuales (D14 §9.2).

Reportar G1/R2/E por modelo, familia y regla; tokens, llamadas, tiempos censurados y distancia de edición como secundarios. Intervalos descriptivos del 95% mediante 10.000 remuestreos por tarea y semilla dentro de modelo, con semilla registrada. No atribuir a dos modelos una población universal de agentes. Si falta una etapa exigida por la tarea, resultado no evaluado y denominador explícito; no contar aceptación de parser como ejecución funcional.

Si falla un umbral, localizar reglas y confusiones, corregir **B**, añadir regresiones y repetir tareas afectadas y sus interacciones con semillas nuevas registradas. No borrar muestras anteriores ni reducir contratos. No se construyen A/C para desempatar. El informe conserva pendientes hasta disponer de evidencia; una medición desfavorable no cambia automáticamente la sintaxis oficial.

La comprensión humana posterior conserva las cohortes Python/Rust/C# de AC §9.6, sin autores del compilador y separadas de participantes confirmatorios F6. Para B: piloto de tres por cohorte; primera muestra nueva de 36 por cohorte, ampliación única a 60 si hay alerta. Doce reglas, 36 preguntas y dos modificaciones por persona. Alertas: comprensión <60%, cambios correctos <50%, brecha entre cohortes ≥30 puntos, o tiempo restringido medio ≥2 veces con brecha de fallos ≥20 puntos; por regla, acierto <50% o confusión modo/nombre ≥30%. Tiempo restringido: máximo 900 segundos por modificación, 900 para fallo/abandono. Informar intervalos por persona, resultados y comentarios; investigar patrones reproducidos por tres personas, con dos revisores de la causa. La incertidumbre no prueba equidad. Ajustar material, diagnóstico o regla de B, sin reabrir las alternativas descartadas. Se conservan las puertas independientes del roadmap y no se anuncian resultados que aún no se han medido.
