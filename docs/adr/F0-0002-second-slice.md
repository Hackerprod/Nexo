# F0-0002 — Llamadas, IR tipada, efectos y préstamos locales

Estado: **decisiones de arquitectura enmendadas; pendiente de revisión independiente**.
Responsable: Codex, rol arquitecto. Fecha: 2026-10-20.  
Perfil normativo enmendado: `f0-0002-v2` (2026-10-22).

Este documento no acredita implementación, ejecución de pruebas, PASS ni aprobación de F0. No modifica `docs/baseline/`, no crea una revisión 0.3 y no sustituye las puertas del roadmap.

La enmienda fija una única semántica objetivo; las diferencias del código existente son obligaciones pendientes, no variantes aceptadas. `f0-0002-v1` es un antecedente histórico de cuotas, no un modo alternativo del lenguaje. El perfil nuevo identifica las unidades dispersas y el límite global de §11.1; conserva el namespace diagnóstico y el modelo de efectos.

**Superficie: sujeto a F0-D15.** La única sintaxis oficial es la candidata B, especificada por F0-D15. Las grafías, EBNF, tokens, ediciones, offsets y tamaños en bytes de ejemplos conservados aquí son referencias documentales anteriores: todo esperado literal dependiente de esas grafías queda **sujeto a F0-D15**, sin autorizar una segunda sintaxis ni compatibilidad. Se conservan reglas semánticas, códigos y obligaciones; la migración recalcula los literales desde la fuente oficial, sin obtener esperados del compilador probado.

## 1. Autoridad, objetivo y división del trabajo

Autoridad: [roadmap rev 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), P01–P09, §2, D01–D10, F0/F1, EX01–EX04, T01–T28 y §6.1; [acuerdos F0](../F0_ACUERDOS.md); [F0-0001](F0-0001-first-slice.md); [D11](F0-D11-closures.md), [D12](F0-D12-async.md), [D13](F0-D13-expresiones-modulos.md) y [catálogo único](F0-CODIGOS.md).

Se toma como referencia de los ocho pendientes documentales la [revisión D11–D13](../validation/F0-D11-D13-design-review.md). Sus resultados no validan este ADR. `HANDOFF.md` conserva una indicación anterior a la redacción de esos ADR; `Pending.md` y el informe posterior registran el diseño existente.

El objetivo es combinar valores independientes, propiedad explícita, préstamos locales, alternativas tipadas y efectos transitivos con reglas que un agente pueda inspeccionar y reparar. Una llamada debe explicar qué presta, copia, consume, modifica y limpia, sin inferir permisos a partir de conveniencias del intérprete.

Se divide el trabajo:

| Corte | Compromiso |
|---|---|
| **F0-0002** | Funciones síncronas directas; IR bajada y tipada; préstamos durante llamadas; `inout`; propiedad por caminos; `Result`/`?`; efectos básicos; terminadores D13; `if` con `value:`; herramienta, actualización de datos y petición reducidas con primitivas simuladas. |
| **F0-0003, pendiente de ADR propio** | Closures locales con préstamos retenidos y generadores síncronos propietarios; `Option`, selección necesaria para consumir `next` y EX04 reducido. No se implementa esta fila únicamente con el presente documento. |

La división evita introducir a la vez préstamos retenidos, estados suspendidos y una nueva representación ejecutable. F0-0002 establece las obligaciones que F0-0003 deberá conservar; no simula `yield` mediante una función ordinaria ni materializa anticipadamente una secuencia para anunciar soporte de generadores.

## 2. Alcance exacto

### 2.1. Incluido

- Un archivo y un módulo privado, con varias funciones ordinarias, llamadas adelantadas y recursión directa o mutua.
- Parámetros tipados de lectura, `inout` y `consume`; argumentos posicionales o nombrados.
- Resultados anotados. La omisión de `->` solo se admite para funciones de resultado unidad en este perfil.
- `i64`, `bool`, `()`, arrays de datos, dos recursos simulados y `Result<T,E>`.
- `let`, `var`, bloques, asignación, retorno explícito, `if`, `while` y expresiones descartadas.
- `if` como expresión con `value:` y unión conservadora de propiedad.
- Copia física de datos; transferencia explícita con `consume`; destrucción explícita y de ámbito.
- Préstamos inferidos de lectura y préstamos exclusivos `inout`, limitados a la llamada.
- `Result`, `Ok`, `Err`, propagación `?` y aritmética fallable `try_add`.
- Los contratos `no_alloc`, `no_block`, `no_suspend` y `no_io`, inferidos transitivamente y comprobados nuevamente después del lowering.
- IR con identidades resueltas, tipos, procedencias, permisos, transferencias, efectos, limpieza y origen fuente.
- Diagnósticos estructurados, explicación consultable y reparaciones comprobadas cuando se anuncien como aplicables.

### 2.2. Diferido

No entran closures, valores de función almacenables, llamadas indirectas, traits, genéricos de usuario, `match`, `Option`, tuplas, `struct`, enums de usuario, módulos adicionales, `pub`, imports, extensiones, atributos, `comptime`, defaults, cadenas, interpolación, flotantes, rangos, `for`, `break`, `continue`, métodos, `+=`, `async`, generadores, regiones, FFI, CoW, RC ni publicación concurrente.

`Result` y `Array` son constructores intrínsecos de tipos; su soporte no acredita genéricos ni traits.

Los arrays solo contienen datos copiables e independientes. Un `Result` sí puede contener recursos, también mediante otros `Result`. Arrays que contengan recursos transitivamente quedan fuera del perfil.

La exclusión es del prototipo, no del lenguaje completo ni de los archivos de aceptación de la base. Una característica reconocida pero diferida usa `NX-E0006`, con `feature_id`, perfil y fase pendiente. No se presenta ese rechazo como prueba de propiedad, suspensión o coherencia.

### 2.3. Correspondencia con D11–D13

De D11 se adoptan los contextos explícitos de lectura, copia y transferencia, la separación entre propiedad y permiso y la regla de `drop`. Se difieren sus entornos de closure.

De D12 se conserva la separación entre operaciones, efectos latentes y limpieza. No se implementan futuros, tareas, cancelación ni `SuspendSafe`.

De D13 se implementan terminadores, `value:` contextual, `if`, tipos exactos, disponibilidad por caminos y reinicialización completa de `var`. Se difieren patrones, módulos y contratos escritos de procedencia.

## 3. Gramática delta sobre F0-0001

Las siguientes producciones sustituyen las correspondientes del primer corte. Las cadenas entre comillas son terminales. No se añade multiplicación al lenguaje de este perfil.

```text
Program       = {Function} EOF
Function      = "fn" Name "(" [Parameters] ")"
                ["->" Type] [WithClause] Block
Parameters    = Parameter {"," Parameter}
Parameter     = ["inout" | "consume"] Name ":" Type

Type          = "i64" | "bool" | "()"
              | "MockResource" | "MockBlockingResource"
              | "Array" "<" Type ">"
              | "Result" "<" Type "," Type ">"

Block         = "{" {Statement} "}"
Statement     = Binding | Assignment | ReturnStmt | ValueStmt
              | IfForm | WhileStmt | Block | ExprStmt

Binding       = ("let" | "var") Name [":" Type] "=" Expr Terminator
Assignment    = Place "=" Expr Terminator
ReturnStmt    = "return" [Expr] Terminator
ValueStmt     = ValueName ":" [Expr] Terminator
WhileStmt     = "while" Expr Block
ExprStmt      = CompareExpr Terminator

Expr          = IfForm | CompareExpr
IfForm        = "if" Expr Block ["else" (Block | IfForm)]
CompareExpr   = AddExpr [("<" | "==") AddExpr]
AddExpr       = TryExpr {("+" | "-") TryExpr}
TryExpr       = AccessExpr ["?"]
AccessExpr    = ["consume" | "inout"] Postfix
Postfix       = Primary {Index | Arguments}
Index         = "[" Expr "]"
Arguments     = "(" [Argument {"," Argument}] ")"
Argument      = [Name ":"] Expr
Primary       = IntLiteral | "true" | "false" | Name
              | "(" [Expr] ")" | "[" [Expr {"," Expr}] "]"

Place         = PlaceRoot {Index}
PlaceRoot     = Name | "(" Place ")"
IntLiteral    = ["-"] Digits
Digits        = Digit {Digit}
Digit         = "0" | "1" | "2" | "3" | "4"
              | "5" | "6" | "7" | "8" | "9"
Name          = ASCIIIdentifier
ValueName     = Name
Terminator    = ";" | InsertedTerminator | MissingTerminator
```

`ValueName` requiere el lexema exacto `value`, seguido de `:` en la misma línea y al comienzo de una sentencia. `value` sigue siendo un identificador ordinario.

`WithClause` se importa de la única producción de D11 §5.2. No se crea una gramática alternativa. El parser reconoce sus listas simples, mapas, operaciones y formas inválidas reconocedoras. Para una función ordinaria solo es válida la operación `call`. `no_heap_alloc` se reconoce, pero queda fuera de este perfil con `code = NX-E0006`, `subcause = null` y `feature_id = no_heap_alloc`.

Las formas vacías y repetidas conservan los diagnósticos y prioridades `NX-E0417` del catálogo. `with { resume(no_io) }` produce `invalid_operation`, antes de una supuesta ausencia de `call`.

Restricciones adicionales del perfil:

1. El destino de una llamada debe resolver una declaración de función o un intrínseco conocido. No hay despacho mediante un valor almacenado.
2. `inout` sin llamada se admite como argumento sobre un `Place`. Debe corresponder a un parámetro `inout`.
3. `consume` transfiere un binding completo, incluso entre paréntesis. Transferencias parciales quedan diferidas.
4. Un prefijo sobre una llamada completa, como `consume f()`, pertenece a los modos de receptor de D11 y queda diferido. No se reinterpreta como transferencia del resultado.
5. Una omisión de resultado que requiere inferir un resultado distinto de unidad recibe `code = NX-E0006`, `subcause = null`, `feature_id = inferred_non_unit_result`. No se escribe el `feature_id` como sufijo de código/subcausa.
6. `Ok`, `Err` y los nombres mock son identificadores de declaraciones intrínsecas del preámbulo, no palabras reservadas. La resolución determina su comportamiento.
7. `MissingTerminator` es recuperación errónea: nunca permite aceptar un programa.
8. Las formas reconocidas para explicar una exclusión no se incorporan a la gramática aceptada.

### 3.1. Léxico y terminadores

Los spans conservan offsets sobre los bytes originales UTF-8. CRLF cuenta como un salto lógico y conserva sus dos bytes. Los comentarios `//` no eliminan la información de salto.

Se aplica D13 §2:

- Un salto termina una expresión completa fuera de paréntesis o corchetes abiertos de esa sentencia, salvo continuación por tokens.
- En este perfil, `(`, `[`, `?`, `+`, `-`, `<`, `==` y la coma de una lista abierta permiten la continuación que corresponda.
- Las continuaciones reconocidas de características diferidas, como `.` o `..`, llegan al diagnóstico de alcance; no se utilizan para fabricar un terminador.
- Una llave `}` en la misma línea no inserta `;`.
- `return` seguido inmediatamente de salto termina un retorno vacío, incluso ante `(`.
- `value:` seguido inmediatamente de salto termina un marcador vacío.
- Para continuar esos operandos se abre `(` antes del salto.
- La apertura de cuerpo tras una firma, y `else` tras su rama, mantienen la construcción estructural aunque estén en otra línea.
- Una sentencia estructural termina con su cierre. Una expresión que contiene ese cierre sigue necesitando su propio terminador.
- No hay retorno implícito ni valor implícito de bloque.

`NX-E0005` señala el siguiente token, o el rango vacío de EOF, con `expected_tokens = [";"]` y `expected_tokens_kind = canonical_insertion`.

### 3.2. NB2: frontera entre decimal y rango

Se reconoce el token decimal reducido:

```text
DecimalToken = Digits "." Digits
RangeToken   = ".."
```

El punto decimal solo se incorpora cuando le sigue un dígito y no otro punto. Por tanto:

- `0.5` es un token decimal reconocido; al encontrarlo como expresión se rechaza con `NX-E0006`, `feature_id = float_literal`.
- `0..n` son entero `0`, rango `..` y nombre `n`; se rechaza el rango con `feature_id = range`.
- `0 . 5` no es un decimal.
- El signo `-` es un token separado; los espacios no deciden entre resta y signo.
- Exponentes, sufijos y semántica flotante completa quedan pendientes.

El inventario de símbolos tokenizables del perfil es cerrado: `(`, `)`, `{`, `}`, `[`, `]`, `,`, `:`, `;`, `?`, `+`, `-`, `<`, `>`, `=`, `@`, `.`, `*`, `..`, `+=`, `!=`, `->`, `=>`, `==`, `<=` y `>=`. Se aplica coincidencia máxima, con la separación contextual de `>=` de §3.6. Las demás categorías son identificadores/palabras reservadas, enteros, el decimal reducido, cadenas entre comillas dobles y EOF; espacios, saltos y comentarios `//` son trivia. Reconocer un token no implica aceptar su construcción. Fuera de cadenas y comentarios, `/`, `%`, `!`, `&` y `|` aislados producen NX-E0001 sobre ese carácter; `/` solo inicia trivia cuando forma `//`, y `!` solo forma un símbolo válido dentro de `!=`. No se reconoce comentario `/*...*/`. El lexer no emite anticipadamente un error de alcance por una cadena o decimal situado después de otro introductor diferido: el parser informa la primera exclusión que encuentra en su recorrido. Los errores léxicos auténticos conservan su etapa.

F03/F04 de §14 fijan los spans de decimal y rango. Esta regla no decide redondeo, valores especiales ni toda la gramática numérica futura.

### 3.3. Identificadores y palabras reservadas

`ASCIIIdentifier` usa coincidencia máxima del patrón `[A-Za-z_][A-Za-z0-9_]*`, excluyendo las palabras reservadas enumeradas aquí. La enumeración es del perfil experimental, no una declaración adicional sobre toda la base.

| Clase | Enumeración cerrada |
|---|---|
| Reservadas de construcciones incluidas | `fn`, `let`, `var`, `return`, `if`, `else`, `while`, `true`, `false`, `inout`, `consume`, `with`. |
| Reservadas para reconocer construcciones diferidas | `match`, `for`, `break`, `continue`, `gen`, `yield`, `async`, `await`, `region`, `struct`, `enum`, `type`, `trait`, `impl`, `where`, `dyn`, `mod`, `use`, `pub`, `extend`, `extension`, `comptime`, `const`, `self`, `super`, `package`. |
| Reservada de migración | `move`. |

Los introductores diferidos **sí están reservados**. En la posición de su construcción producen el diagnóstico de alcance correspondiente. En una posición que exige un nombre, por ejemplo `let match = 1;`, producen `NX-E0002` con el token reservado como inesperado y `Name` como categoría esperada; no se interpreta ese binding como una selección por patrones. En posición de expresión, `self` reconoce una referencia de receptor diferida: NX-E0006, `feature_id = methods`, etapa `parse`, span del token `self`. En una posición que exige Name, como `let self=1;`, conserva NX-E0002, sin inventar una declaración de receptor.

Son nombres ordinarios, reconocidos contextualmente cuando corresponde: `value`, `from`, `result_from`, `retains`, `borrow`, `copy`, `as`, `in`, `create`, `call`, `resume`, `cancel`, `drop`, `unrestricted` y los cinco nombres `no_*` de D11. También lo son los nombres de tipos y del preámbulo, incluidos `i64`, `bool`, `Array`, `Result`, `MockResource`, `MockBlockingResource`, `Ok`, `Err` y las primitivas mock. No se heredan las exclusiones de nombres intrínsecos del parser de F0-0001.

### 3.4. Reconocimiento de tipos fuera del perfil

La producción aceptada `Type` es cerrada. En posición de tipo se reconoce además, únicamente para diagnóstico, una ruta de nombres con posibles argumentos de tipo.

- `Option` conserva `feature_id = option`.
- Los tipos reconocidos de futuros/tareas quedan en `feature_id = async`.
- Un tipo no incluido y sin exclusión más específica, como `u32`, `u64`, `f32`, `String` o `app.Error`, produce `NX-E0006`, `feature_id = unsupported_type`.
- En este último caso el span abarca la ruta de tipo completa, por ejemplo `app.Error`, y el diagnóstico registra su grafía. No se emite `NX-E0101` como si fuera un binding de expresión desconocido.
- Un delimitador ausente o una lista de tipos mal formada sigue siendo un error sintáctico. El reconocimiento de alcance no acepta firmas incompletas.

La prioridad depende del contexto: `app.Error` en posición de tipo usa `unsupported_type`; un `use` de módulo usa `modules_visibility`; la selección de miembro de un valor usa `methods`. No se emiten las tres causas por una misma construcción.

`indirect_call` se retira de la lista de `feature_id` emitibles de este perfil: no existe un valor invocable aceptado que permita alcanzar esa regla de manera independiente. La creación de ese valor ya falla por `closure` o `function_value`; un valor aceptado no invocable usa `NX-E0111/non_callable_target`. No se exige una fixture imposible ni se atribuye cobertura de llamadas indirectas.


### 3.5. Signo de literal y operadores reconocidos sin producción aceptada

`IntLiteral = ["-"] Digits` es una producción del **parser**. El lexer entrega `-` y los dígitos por separado. Cuando se espera un operando, el parser reúne un `-` seguido de un token entero en un único literal firmado; admite la trivia permitida entre ambos. El rango `i64` se comprueba sobre el valor firmado completo, por lo que `-9223372036854775808` es representable. El span comprende desde el signo hasta el final de los dígitos.

Después de un operando completo, `-` es resta binaria. Por tanto, `value -1` y `value - 1` tienen el mismo árbol pertinente. No se incorpora negación general de expresiones: `-x` no es un literal firmado aceptado.

Se decide conservar **NX-E0002** para `*`, `!=`, `>`, `<=` y `>=` en posición de operador binario. Son tokens reconocidos, pero no pertenecen a una producción aceptada ni tienen `feature_id` en este perfil. No se amplía por ellos la lista de NX-E0006.

El parser reconoce específicamente ese intento de operador antes de diagnosticar un terminador ausente derivado. Exige:

```text
stage = parse
parser_context = unsupported_binary_operator
expected_tokens = ["+", "-", "<", "=="]
expected_tokens_kind = admissible_set
```

El span primario es el operador completo; `recognized_prefix` comprende el prefijo de la sentencia hasta el comienzo del operador. La lista anterior describe los operadores binarios admisibles en ese contexto reconocedor, no todos los tokens que podrían seguir a una expresión en otros contextos.

S27/S28 de §14.7 concretan ambas decisiones. No se atribuye al prototipo semántica de multiplicación o desigualdad.


### 3.6. Operadores excluidos y operandos de los prefijos

Los cinco operadores excluidos de §3.5 se reconocen antes de considerar un terminador ausente. También impiden insertar un terminador ante su continuación en la siguiente línea. Por tanto, `a > b`, `a <= b` y `a >= b` producen NX-E0002 con `parser_context = unsupported_binary_operator`, los mismos cuatro operadores esperados de §3.5 y span del operador completo. No se propone insertar `;` para esos casos.

El lexer reconoce `<=` y `>=` como símbolos de dos bytes contiguos. En posición de cierre de argumentos de **tipo**, el parser puede separar el token `>=` en `>` y `=`, conservando sus spans de un byte. Esta separación solo ocurre cuando la producción de tipo necesita ese `>`; no se aplica a una comparación. Así se conserva la fuente ya publicada `let r:Result<i64,i64>=Ok(1);`. Los espacios no unen tokens separados para fabricar `<=` o `>=`.

Después de quitar agrupaciones, un `Place` tiene una raíz Name y cero o más índices. Los prefijos se clasifican en este orden:

1. Un prefijo aplicado a una llamada completa conserva NX-E0006, `feature_id = receiver_mode`.
2. Un operando que no tiene forma de `Place` produce NX-E0002 en `parse`. `parser_context` es `consume_place` o `inout_place`; `expected_tokens = ["Place"]`; `expected_tokens_kind = admissible_set`.
3. `consume` sobre un `Place` proyectado conserva NX-E0006, `feature_id = partial_transfer`.
4. Un binding completo para `consume`, o un lugar para `inout`, pasa a las comprobaciones de nombre, contexto, modo y permiso correspondientes.

En el punto 2 se señala el primer token que impide derivar `Place`: por ejemplo, `1` en `consume 1`, `inout 1` o `consume (1+2)`. `recognized_prefix` empieza en el prefijo y termina al comienzo de ese token, incluyendo la trivia y los paréntesis ya reconocidos. No se inventa una transferencia de un literal ni un préstamo mutable de un temporal.

Estas reglas no cambian que un temporal nuevo pueda entregarse directamente a un parámetro `consume`, sin escribir el prefijo.
## 4. Tipos, nombres y llamadas

### 4.1. Resolución e inferencia

Se registran todas las firmas antes de comprobar cuerpos. Una función puede llamar a otra declarada después. No hay sobrecarga por retorno ni por argumentos.

Las funciones nombradas y los bindings locales siguen un espacio de resolución determinista. Los locales pueden sombrear nombres exteriores. Si un local escalar sombrea una función y se intenta invocarlo, corresponde `NX-E0111/non_callable_target`; no se salta el binding para encontrar otra declaración.

Los parámetros se anotan. Los resultados no unitarios se anotan. Se infieren bindings y se propaga contexto esperado desde anotaciones, parámetros, asignaciones y retornos.

`Ok(1)` necesita determinar también el tipo de error. `Err(7)` necesita determinar también el tipo de éxito. No se elige `i64`, unidad ni un tipo comodín para completar un hueco. Si el contexto local no lo resuelve, se emite `NX-E0113`. Un array vacío sin tipo de elemento conserva `NX-E0103`.

`Result<T,E>` es copiable e independiente solo si ambas alternativas lo son. La propiedad es del tipo completo: conocer que una expresión concreta construye `Ok` no permite copiar un tipo cuyo `Err` pueda contener un recurso.

### 4.1.1. Inferencia monomórfica diferida

Cada binding sin tipo determinado recibe una variable de tipo monomórfica. Anotaciones, inicialización, asignaciones, argumentos, operadores y retornos añaden restricciones de igualdad y dominio; se unifican transitivamente, sin generalización ni un tipo nuevo por uso. En `var w=v`, la igualdad entre los tipos de `w` y `v` persiste tras resolver cualquiera de ellos. Sus valores siguen siendo copias independientes (§6.1): compartir restricciones no comparte almacenamiento ni propiedad.

Se distinguen tres estados internos:

| Estado | Significado y tratamiento |
|---|---|
| `Never` | La expresión no continúa normalmente; fondo del retículo. `join(Never,T)=T`; no añade restricciones concretas ni exige un valor/temporal de esa salida. |
| Tipo por inferir | Variable monomórfica aún no resuelta; conserva sus restricciones y relaciones. No equivale a divergencia ni a `Never`. |
| Resultado contaminado | Depende de un error previo. Conserva la causa y suprime solo diagnósticos derivados; no demuestra igualdad ni borra errores independientes. |

Un inicializador divergente sin anotación no restringe el tipo del binding: el análisis hipotético posterior usa su variable de tipo, sin convertir cada lectura en otra divergencia. Una anotación sí restringe ese binding. `missing_branch_value` es una obligación estructural pendiente, no una variable libre que permita aceptar un valor inexistente. La aceptación se determina con todas las restricciones locales, también en código hipotético; no depende del orden interno del solucionador.

Se conserva la procedencia de las restricciones. Para elegir un diagnóstico reproducible se considera su orden fuente: la primera restricción incompatible con las precedentes se denuncia en su operando, con el contexto esperado de esa restricción; los empates siguen el orden de hijos. Esta selección de evidencia no obliga a resolver internamente en ese orden. Los constructores no resueltos conservan NX-E0103/NX-E0113 (§13.17); no se inventa un tipo concreto. Una variable usada solo hipotéticamente puede quedar interna sin materializar un valor o slot ejecutable de tipo desconocido.

Se formalizan `adr_gap_never_bottom` y `adr_gap_deferred_var_alias`: `(DV)==true` y `true==DV` son simétricos en aceptación y restricciones, siendo `DV` una expresión sin continuación; `var v=DV;var w=v;v=3;w=true;` se rechaza, igual que su equivalente vivo. §14.10 fija fuentes y esperados literales. `Never`, variables pendientes y contaminación nunca se serializan en `data`, `expected_type` ni `found_type`.

### 4.2. Asociación y orden de argumentos

Primero se comprueba la asociación estructural, sin ejecutar nada:

- Un argumento posicional ocupa el siguiente parámetro.
- Tras el primer argumento nombrado, todos los restantes son nombrados.
- Cada parámetro aparece exactamente una vez.
- Las etiquetas coinciden con nombres de parámetros.
- No hay defaults en este perfil.

Se diagnostican etiqueta desconocida, argumento duplicado, argumento ausente, exceso posicional, posicional posterior a nombrado y modo incompatible mediante las subcausas de `NX-E0111`.

La evaluación ocurre **en el orden escrito**, una sola vez, antes de asociar los valores preparados a los slots del frame. No se reordena por etiquetas.

### 4.3. Modos y propiedad del argumento

| Parámetro | Argumento y operación |
|---|---|
| Ordinario `x: T` | Un lugar existente se presta para lectura. Un temporal propietario se conserva en el llamador y se presta durante la llamada. |
| `inout x: T` | Exige `inout lugar`, origen modificable y permiso exclusivo. |
| `consume x: T` | Recibe propiedad. Un nombre sin `consume` solicita copia lógica; si no es copiable, `NX-E0201`. `consume nombre` transfiere y lo invalida. Un temporal nuevo se entrega directamente. |

No se deduce una transferencia porque el parámetro sea `consume`.

`consume p` también puede producir un temporal que se preste a un parámetro ordinario: el origen queda consumido y el temporal se destruye después de la llamada. Es una elección explícita del programa.

Un parámetro ordinario no puede mutarse, consumirse ni destruirse como propietario. Un parámetro `inout` permite reemplazar el valor, pero no extraerlo dejando un hueco ni consumir el origen prestado. Esos aumentos de permiso producen `NX-E1105/permission_escalation`.

El binding de un parámetro `consume` es propietario e inmutable, como `let`; se puede consumir o destruir.

### 4.4. Entrada ejecutable

`check` comprueba módulos, incluidos módulos sin `main`. `run` añade la comprobación de entrada:

- Debe existir una declaración `main`.
- No recibe parámetros.
- Su resultado es `()` o `Result<(), i64>`.

La ausencia o firma incompatible usa `NX-E0112`. No se confunde una biblioteca sin entrada con un cuerpo inválido.

`check` y `run` comparten todas las comprobaciones anteriores a esta puerta. Ningún fallo de comprobación ejecuta primitivas.


### 4.5. Duplicación y destinos no declarados

Se amplía expresamente el alcance de dos entradas existentes del catálogo, sin crear subcausas:

| Situación | Diagnóstico | Span y datos |
|---|---|---|
| Dos funciones del archivo con el mismo nombre | `NX-E0104`, `subcause = null` | Nombre de la segunda declaración; primera declaración relacionada; `declaration_kind = function`. |
| Dos parámetros de una función con el mismo nombre | `NX-E0104`, `subcause = null` | Nombre del segundo parámetro; primero relacionado; `declaration_kind = parameter`. |
| Binding duplicado en el mismo ámbito | `NX-E0104`, `subcause = null` | Nombre del segundo binding; primero relacionado; `declaration_kind = binding`. |
| Destino de llamada no declarado | `NX-E0101`, `subcause = null` | Nombre del destino; `use_kind = call_target`. |
| Nombre de valor no declarado | `NX-E0101`, `subcause = null` | Nombre usado; `use_kind = value`. |

Los parámetros ocupan el ámbito inicial del cuerpo de la función. Un binding del cuerpo exterior no puede redeclarar un parámetro; un bloque hijo sí puede sombrearlo. Las funciones del archivo ocupan el ámbito del módulo; el preámbulo es exterior y puede sombrearse sin alterar la identidad ni los contratos de sus intrínsecos.

La duplicación de funciones se comprueba al registrar nombres y la de parámetros al registrar la firma. No se escoge una declaración arbitraria para continuar. Un nombre resuelto a un escalar no es un nombre ausente: su invocación conserva `NX-E0111/non_callable_target`.

### 4.6. Comparaciones

Ambos operandos se evalúan una vez, de izquierda a derecha, en contexto de lectura. Comparar no copia arrays ni consume valores.

| Operador | Tipos admitidos | Resultado y regla |
|---|---|---|
| `<` | Dos `i64` | `bool`; orden firmado de `i64`. |
| `==` | Dos valores del mismo tipo de datos copiable: `i64`, `bool`, `()`, `Array<T>` o `Result<T,E>`, transitivamente | `bool`; igualdad estructural, sin coerciones. |

Para `==`, los arrays comparan primero longitud y después elementos por índice creciente, con cortocircuito. Los `Result` comparan primero la variante y solo comparan el payload si coincide. Unidad es igual a unidad. Cada par de celdas visitado cobra dos unidades de trabajo, una por cada celda; longitud o etiqueta distinta evita visitar los payloads restantes.

No se admite `<` sobre `bool`, unidad, arrays ni `Result`. Tampoco se admite `==` sobre recursos ni sobre un `Result` cuyo tipo pueda contenerlos, aunque se conozca una variante concreta sin recurso. Un array con recursos ya queda excluido por `resource_array`.

Si ambos operandos aportan tipos concretos y estos difieren, corresponde `NX-E0102/type_mismatch` sobre el operando derecho. Si coinciden pero el operador no admite ese tipo, corresponde el mismo código/subcausa sobre el operador, con el dominio exigido y el tipo encontrado. No se intenta copiar el recurso para producir antes `NX-E0201`. Un operando `Never` no impone igualdad concreta: se comprueba el dominio del otro en cualquiera de las posiciones. Los operandos posteriores a divergencia se comprueban hipotéticamente y conservan errores independientes.

`assert_eq` utiliza el mismo dominio y recorrido de igualdad de datos; una desigualdad de valores aceptados produce `R0201`.

### 4.7. Contexto esperado y tipos de ramas

La ausencia de tipo esperado es distinta de recibir un tipo desde una anotación, asignación, parámetro o retorno:

- Sin tipo esperado, cada continuación de un `if` sintetiza su tipo. Una discrepancia entre continuaciones produce `NX-E1303` sobre el valor de la primera continuación posterior incompatible.
- Con tipo esperado, ese tipo se propaga a los operandos de `value:`. Una incompatibilidad concreta produce `NX-E0102/type_mismatch`; no se duplica con `NX-E1303` por la misma causa.
- Los defectos estructurales de `value:` y de terminación se diagnostican antes de incompatibilidades derivadas de un valor inexistente.

B03 utiliza deliberadamente un binding sin anotación. B09 conserva el caso con retorno `i64` y exige `NX-E0102/type_mismatch`. B01/B02/B04–B08 no contienen valores con tipos incompatibles que compitan con su defecto estructural.


### 4.8. Operadores intrínsecos frente a D08

La resolución intrínseca de `+` y `-` sobre `i64`, de `<` exclusivamente sobre `i64` y de `==` sobre datos, incluidos Array y Result, es una **simplificación explícita del prototipo respecto a D08**. D08 establece resolución de operadores mediante traits; este corte todavía no implementa ese mecanismo. Se conservan los desbordamientos comprobados de suma y resta y los dominios de comparación publicados.

Los dominios cerrados de §4.6 no acreditan `PartialEq`, otros traits de operadores, implementaciones de usuario ni resolución genérica. La futura sustitución por traits deberá conservar las reglas de lectura, orden y efectos que correspondan, sin atribuir retrospectivamente ese soporte a F0-0002.

## 5. Préstamos reducidos y exclusividad

### 5.1. Procedencia

Una procedencia identifica el propietario lógico, su ámbito o frame y una ruta de proyección. Nunca se identifica exclusivamente por el texto del nombre ni por una dirección de memoria de Rust.

Se distinguen:

- Propietario local.
- Temporal propietario.
- Parámetro propietario.
- Préstamo de lectura de parámetro.
- Préstamo exclusivo de parámetro.
- Représtamo subordinado a otro permiso.

Los préstamos de un argumento se adquieren cuando se evalúa ese argumento y permanecen hasta después de la limpieza del callee. Por tanto, argumentos posteriores también deben respetarlos.

### 5.2. Regla de solapamiento

El análisis inicial considera solapadas todas las proyecciones de la misma raíz. Dos índices distintos, incluso constantes, no acreditan disjunción.

- Lectura con lectura: compatible.
- Lectura con `inout`: incompatible.
- Dos `inout` de la misma raíz: incompatible.
- Consumo o destrucción del propietario con un préstamo vivo: incompatible.

El segundo préstamo mutable solapado usa `overlapping_mutable_borrows`. Los demás conflictos con un permiso vivo usan `active_borrow_conflict`.

`f(inout a[0], inout a[1])` se rechaza conservadoramente. El diagnóstico explica que el perfil no demuestra disjunción de elementos.

### 5.3. Représtamos

Una función con `inout x` puede prestar temporalmente `x` a otra función, por lectura o mediante `inout`. Mientras el représtamo está activo, el permiso padre queda suspendido para accesos incompatibles. Se restaura al finalizar la llamada subordinada, también si esta propaga un error.

Dos parámetros no se consideran automáticamente raíces independientes: en el llamador se comprueba su relación; en el callee se conservan las obligaciones de no solapamiento de su firma. No hay callbacks ni globals mutables que puedan introducir aliases ocultos en este perfil.

### 5.4. No escape

Ningún tipo aceptado almacena préstamos. Un parámetro ordinario usado como inicializador, elemento, resultado de rama o retorno entra en contexto propietario:

- Si es dato copiable, produce una copia independiente.
- Si contiene recursos, copiarlo da `NX-E0201`.
- Intentar transferirlo con `consume` da `permission_escalation`.

No hay conversión de un préstamo en propietario ni extensión implícita de vida. Un resultado de función no contiene permisos a slots de un frame finalizado.

Esta es una garantía del subconjunto sin portadores de préstamos, no evidencia de las reglas generales `from`, `retains`, `NX-E1104` o `NX-E1318`. El escape mediante closure, vista, error prestado o tarea continúa NO EVALUADO.

## 6. Evaluación, ramas, bucles y limpieza

### 6.1. Valores y asignación

La lectura de un array como argumento ordinario no copia el array. Su copia en contexto propietario sigue siendo física y profunda.

Una indexación sobre un lugar presta la base para calcular el acceso. Si el resultado se necesita como propietario, copia solo el elemento seleccionado. Esto sustituye la copia del array completo que realizaba el intérprete anterior al evaluar una lectura indexada.

La asignación mantiene la precedencia comprobada en F0-0001:

1. Resolver la identidad del binding destino.
2. Evaluar todos los índices, de izquierda a derecha, conservándolos como `i64`.
3. Evaluar el RHS y obtener su valor propietario.
4. Comprobar disponibilidad, permisos y límites contra el valor actual del destino.
5. Limpiar el valor reemplazado e instalar el nuevo.

No se conserva un puntero a un elemento entre los pasos 2 y 4. Las llamadas del RHS pueden modificar legítimamente la raíz cuando no exista otro préstamo incompatible; los límites se comprueban después de esas modificaciones. Si el RHS consume una raíz necesaria para una asignación indexada, la comprobación de disponibilidad rechaza el programa.

Una asignación completa a `var` puede reinicializar un slot consumido. No hay reinicialización parcial de recursos. Si el RHS falla, no se instala el nuevo valor y el anterior continúa sujeto a la limpieza correspondiente, salvo una transferencia que el propio RHS ya haya efectuado.

### 6.2. `if` y `value:`

Se aplica D13:

- En posición de sentencia, `if` no admite un `value:` destinado a ese bloque.
- En posición de expresión, exige `else`.
- Todo camino que continúa entrega un valor mediante un `value:` directo y final.
- Los tipos de las continuaciones normales coinciden exactamente.
- Un retorno o propagación de error no aporta un valor a la unión.
- No hay coerción, boxing ni selección por copiabilidad.

La unión de disponibilidad contiene los estados de todos los caminos que continúan. Si alguno consumió un binding, un uso posterior da `NX-E0202/maybe_consumed`. Si todos lo consumieron, da `definitely_consumed`.

Los caminos que retornan se excluyen de esa unión, pero conservan efectos y limpieza. Una expresión sin continuación tiene `Never`; se distingue de inferencia pendiente y error previo (§4.1.1). No se deducen correlaciones entre condiciones de dos `if` distintos. Un uso comprobado solo hipotéticamente no exige un valor ejecutable para una expresión `Never`: se suprime una NX-E0202 derivada únicamente de esa ausencia de valor/continuación, sin inventar un consumo. Esto no suprime un consumo explícito independiente ni los errores de tipos, nombres o permisos del código hipotético (§§4.1.1 y 13.9).

### 6.3. `while`

La condición es `bool` y se evalúa antes de cada iteración. La salida por condición falsa conserva el estado que alcanza esa condición.

Los estados de propiedad se resuelven por punto fijo sobre el CFG. Un valor consumido durante una iteración debe haberse reinicializado antes de cualquier camino que vuelva a necesitarlo. No se supone que el bucle ejecutará cero o una iteración por observar una constante.

La condición completa pertenece a la cabecera: sus consumos y reinicializaciones alcanzan tanto el cuerpo como la salida falsa. No se correlaciona el valor booleano de un brazo con un estado de propiedad más preciso. La unión distingue consumo posible y definitivo, excluye retornos y conserva testigos (§13.15). El punto fijo es monótono sobre hechos y testigos finitos, con una ruta canónica por sitio de consumo y cada retroarista a lo sumo una vez por testigo. Termina por convergencia o por §11.1, nunca aceptando estados intermedios.

Los locales de la condición y del cuerpo tienen una instancia nueva por iteración; los del cuerpo se limpian al terminarla. No hay `break` ni `continue` en este perfil.

Las sentencias posteriores a una salida incondicional siguen comprobándose en un estado hipotético separado. Sus errores no se ocultan y sus estados no contaminan las continuaciones alcanzables. Se conserva así la intención conservadora de F0-0001.

### 6.4. Orden de limpieza

Cada adquisición tiene una identidad y exactamente un propietario vivo. Mover cambia el propietario; no adquiere otro recurso.

Orden:

1. Temporales de una expresión, en orden inverso de inicialización cuando termina su uso.
2. Locales de cada ámbito abandonado, en orden inverso de declaración; solo slots vivos.
3. Parámetros propietarios del callee, en orden inverso de declaración.
4. Liberación de préstamos de la llamada.
5. Temporales propietarios que el llamador había prestado, en orden inverso de evaluación.

Antes de entrar al callee, los argumentos propietarios preparados pertenecen a una zona temporal del llamador. Si falla un argumento posterior, se limpian los ya preparados en orden inverso de evaluación. Solo tras preparar todos se transfieren a los parámetros, instalados en orden de declaración.

El resultado de retorno se prepara en un slot propietario separado. Después se limpian los ámbitos del callee. El resultado se entrega una vez y no se destruye como local. Si la evaluación del resultado falla antes de completarse, se limpian únicamente sus componentes inicializados.

Las alternativas de `Result` solo destruyen el payload activo. No hay destructor de usuario ni destructor que propague `?`.

### 6.5. NB1: `drop`

Se declara expresamente esta **ampliación experimental**:

`drop(p)` es la grafía provisional del intrínseco de destrucción de F0. Consume un lugar propietario por su sintaxis, sin intentar copiarlo, y deja el slot consumido. También destruye un temporal propietario.

La operación se aplica a datos, recursos y `Result`; no solo a recursos. Destruir un dato no deja eventos de recurso, pero invalida el binding.

Un préstamo no concede permiso de destrucción. Un símbolo de usuario llamado `drop` es una función ordinaria y no adquiere este privilegio. La resolución del intrínseco, no una comparación textual tardía, decide el lowering.

No se congela esta grafía como API de biblioteca 1.0.


### 6.6. Destrucción de proyecciones

El intrínseco `drop` puede destruir un binding propietario **completo** o un temporal propietario. No deja huecos en un agregado existente.

Por tanto, `drop(a[0])` y `drop((a[0]))`, cuando `a[0]` es un `Place`, producen NX-E0006, `feature_id = partial_transfer`, sobre la proyección completa. Esta ampliación de la descripción de `partial_transfer` incluye destrucción parcial; no añade un `feature_id`. La etapa es `arguments`, después de resolver que el destino es el intrínseco `drop`. Se relaciona el nombre de ese destino con `role = context`.

Una función de usuario llamada `drop` conserva sus reglas ordinarias. Por ejemplo, `fn drop(x:i64){}` puede recibir `a[0]` para lectura.

`drop([1][0])` destruye un temporal: la indexación del array temporal entrega el elemento y limpia el resto según §11.6. No extrae un campo de un binding que continúe vivo. `drop(a)` conserva la destrucción completa; sobre un parámetro prestado sigue exigiendo el permiso que ese parámetro no concede.

## 7. `Result`, propagación y fallos del simulador

### 7.1. Resultado del programa

`Result<T,E>` tiene las alternativas `Ok(T)` y `Err(E)`. No hay conversiones de error implícitas en este perfil.

`e?`:

1. Evalúa `e` una sola vez en contexto propietario.
2. Exige `Result<T,E>`.
3. Exige que la función envolvente retorne `Result<U,E>` con el mismo `E`.
4. En `Ok`, extrae el payload propietario y continúa.
5. En `Err`, prepara ese error como resultado de la función y ejecuta su limpieza.

`r?` copia `r` si su tipo admite copia. Para transferir un `Result` no copiable se escribe `consume r?` o `(consume r)?`; el segundo uso del binding queda rechazado.

El `?` de una rama retorna de la función envolvente, no entrega un `value:` a otra rama. Los efectos anteriores y las modificaciones `inout` ya realizadas no se revierten.

Tipos incompatibles del operando, función envolvente o error usan `NX-E0102/type_mismatch`, con span primario de `?` y tipos relacionados.

### 7.2. Canales separados

Se separan en la API:

- Resultado normal de Nexo, incluido `Err(E)`.
- Fallo experimental del simulador.
- Error de compilación.
- Fallo interno de implementación.

`try_add(a, b)` devuelve `Result<i64,i64>`: `Ok(suma)` o `Err(30)` por desbordamiento. No genera `R0101`.

Se conservan como fallos experimentales:

- `R0101`: desbordamiento de `+`.
- `R0102`: índice inválido.
- `R0201`: fallo de `assert_eq`.
- `R0301`: ejecución de `fail()`.
- `R0001`: agotamiento de trabajo.

Se añade `R0103` para desbordamiento de resta y `R0002` para las otras cuotas dinámicas del perfil.

`?` no captura esos fallos ni los convierte en `Err`. Estos fallos limpian el estado del simulador, pero no constituyen una implementación de la política completa de `panic` de D02.

Un fallo interno, una IR inválida o una transición imposible no se presentan como error del programa. Se interrumpe la ejecución con resultado interno diferenciado. No se promete limpieza frente a OOM, aborto o corrupción del anfitrión.


### 7.3. Grafía y error provisionales de `try_add`

`try_add(a,b)` es una primitiva **provisional del simulador**. Sustituye, dentro de este perfil sin métodos, la grafía `a.try_add(b)` utilizada por la base. No redefine la API definitiva del lenguaje.

Igualmente, `Err(30)` es la codificación experimental de desbordamiento dentro del error `i64` del modelo. No fija el tipo ni la representación del error de la biblioteca real, de `app.Error` o del método de la base. La migración futura deberá conservar la operación fallable y documentar el cambio de interfaz.

## 8. Efectos y primitivas confiadas

### 8.1. Dominio de análisis

Se usan los efectos `{alloc, block, suspend, io}`.

Cada resumen mantiene dos conjuntos:

- Efectos conocidos que pueden producirse.
- Efectos cuya ausencia no está justificada.

La unión conserva ambos. Una prohibición solo se verifica si el efecto está ausente de los dos conjuntos. `unknown_effect` nunca equivale a conjunto vacío.

Una función privada sin cota obtiene un resumen de su cuerpo. Una cota escrita es una obligación, no un sustituto del resumen. `unrestricted` no borra lo inferido.

Las componentes fuertemente conexas de llamadas se resuelven mediante punto fijo monótono. Solo funciones con cuerpo completo disponible pueden comenzar con contribución local vacía. Una primitiva o interfaz sin evidencia suficiente empieza con desconocidos, no con vacío.

El análisis incluye:

- Evaluación de argumentos.
- Copias y construcción de payloads.
- Condiciones y todas las ramas.
- Cuerpos de bucles, aunque su condición sea constante.
- Llamadas transitivas.
- Temporales y todas las salidas de limpieza representadas.
- Fallos experimentales y limpieza por cuotas del simulador.

Incluir las salidas por cuota es conservador: puede impedir un contrato en una función que normalmente solo transfiere un recurso cuyo cierre tiene efectos. No autoriza ejecutar ese cierre en el retorno normal.

La destrucción del valor retornado sigue siendo obligación del nuevo propietario. La IR no añade un `Drop` normal del resultado para justificar un resumen conservador.

### 8.2. Contratos y evidencia

| Contrato | Código |
|---|---|
| `no_alloc` | `NX-E0412` |
| `no_block` | `NX-E0413` |
| `no_suspend` | `NX-E0414` |
| `no_io` | `NX-E0415` |

Cada incumplimiento contiene `contract_origin`, operación, cadena causal y `effect_evidence = known_effect | unknown_effect`.

Una causa transitiva señala exclusivamente el nombre del destino de la llamada del cuerpo comprobado, sin argumentos ni paréntesis, y enlaza cada llamada hasta la operación originaria. La misma convención se aplica a una llamada intrínseca directa y a los pasos de llamada de cause_chain; E04–E06 conservan sus spans publicados. Una limpieza implícita señala el nombre del binding propietario y relaciona la salida y la cota escrita incumplida. El contrato confiado del destructor se identifica en cause_chain, sin inventar un span fuente del manifiesto ni un drop escrito. Para el intrínseco `drop(r)` explícito que incumple una cota, `operation = value.drop` y `primary_span` señala únicamente el nombre `drop`; el paso causal `value.drop`, de kind `drop`, abarca la llamada completa `drop(r)`, sin terminador. No se añade un paso `intrinsic.call` para esa misma destrucción.

Se comprueban los resúmenes antes y después del lowering. Un helper nuevo debe tener efectos conocidos o desconocidos explícitos. La ausencia de metadatos invalida la verificación; no se convierte en vacío.

### 8.3. Representación y significado de `no_alloc`

El modelo de representación es copia física de arrays y payloads propietarios. No hay CoW:

- Construir o copiar un array se clasifica conservadoramente con `alloc`, incluido el constructor vacío.
- Copiar escalares o unidad no reserva.
- Construir el envoltorio inline de `Result` no reserva por sí mismo.
- Leer un elemento escalar prestado no copia el array.
- Modificar un elemento escalar existente mediante `inout` no reserva.
- Reemplazarlo por un array construido o copiado conserva el efecto de esa operación.
- Las liberaciones no son reservas, pero sus destructores conservan los demás efectos.

Los contratos se etiquetan `effect_model = f0-0002-sim-v1`. Certifican el modelo del prototipo y su manifiesto de primitivas. No afirman ausencia de asignaciones de Rust, del compilador, de la instrumentación o de la CLI, ni describen todavía código nativo.

### 8.4. Manifiesto cerrado de primitivas

| Primitiva u operación | Firma o dominio | Efectos del modelo |
|---|---|---|
| `mock_open(label)` | `i64 -> MockResource` | `io`; registra adquisición. |
| `MockResource.drop` | Destrucción propietaria | `io`; cierre infalible simulado. |
| `mock_blocking_open(label)` | `i64 -> MockBlockingResource` | `io`; no bloquea al construir. |
| `MockBlockingResource.drop` | Destrucción propietaria | `block, io`; simula cierre potencialmente bloqueante mediante un evento, sin dormir. |
| `inspect(r)` | Lectura de cualquiera de los dos recursos | `io`; evento de inspección. |
| `drop(v)` | Intrínseco propietario | Resumen de destrucción del tipo y variante activa. |
| `len(a)` | Lectura de `Array<T> -> i64` | Vacío. |
| `try_add(a,b)` | `i64, i64 -> Result<i64,i64>` | Vacío. |
| `assert_eq(a,b)` | Lecturas de datos del mismo tipo | Vacío; puede producir `R0201`. |
| `print(v)` | Lectura de datos -> unidad | `io`; conserva el formato de datos de F0-0001. |
| `fail()` | Unidad, salida experimental | Vacío propio; agrega limpieza de la salida. |
| `mock_read_records(r)` | Lectura de `MockResource -> Result<Array<Result<i64,i64>>,i64>` | `alloc, io`. |
| `mock_write_i64(n)` | `i64 -> Result<(),i64>` | `io`. |
| `mock_request()` | `Result<i64,i64>` | `io`. |
| `mock_reply(status)` | `i64 -> Result<(),i64>` | `io`. |
| `mock_unknown()` | Unidad; sin argumentos ni retención | Desconocidos los cuatro efectos. |

`mock_unknown` es una fixture confiada de información incompleta: su implementación de prueba termina sin efectos observables, pero su resumen no permite demostrar prohibiciones. Una ejecución silenciosa no fortalece ese contrato.

Los eventos mock representan E/S del modelo. No abren archivos, sockets, hilos ni transportes. Ninguna primitiva retiene parámetros, devuelve préstamos o llama código Nexo.

El manifiesto lleva identidad y versión. Los cuerpos de usuario no pueden sobrescribir los contratos de la identidad intrínseca; un nombre sombreado resuelve otra declaración.


### 8.5. Resumen estático de destrucción de `Result`

En la fila `drop(v)` del manifiesto, «variante activa» se refiere a la ejecución. La ejecución destruye exclusivamente el payload presente.

El resumen estático de destrucción de `Result<T,E>` es la unión de los resúmenes de destrucción de **ambas variantes**, incluidos efectos conocidos y desconocidos. Solo puede restringirse a una variante cuando el análisis conserva evidencia explícita de que esa variante es la única posible en ese punto. La ausencia de información de variante exige la unión; no se elige la variante que convenga al contrato.

Conocer una variante para efectos no modifica la propiedad de copiabilidad del tipo completo de §4.1. El verificador comprueba la evidencia de cualquier refinamiento usado por el lowering.

Los nombres y formatos de los eventos del manifiesto, incluida la interpretación de «registra el intento», se fijan en §12.4.

## 9. IR bajada y verificable

### 9.1. Forma

La IR contiene funciones con CFG, bloques básicos, slots tipados y temporales identificados. No es el AST con una bandera de comprobación.

Identidades mínimas: `FunctionId`, `BlockId`, `LocalId`, `TempId`, `TypeId`, `LoanId`, `ScopeId` y `OriginId`. Las rutas incluyen identidad de raíz y proyecciones evaluadas.

Cada instrucción o terminador conserva:

- Span principal y orígenes relacionados.
- Tipo de entradas y salidas.
- Modo de acceso.
- Procedencias y permiso padre cuando hay représtamo.
- Efectos propios y llamadas contribuyentes.
- Estado de inicialización/transferencia.
- Ámbito y obligación de limpieza.

Operaciones mínimas:

```text
Const, CopyValue, TransferValue, ReadPlace,
BuildArray, BuildOk, BuildErr,
CheckedAdd, CheckedSub, Compare, ReadIndex, WritePlace,
BeginReadLoan, BeginExclusiveLoan, EndLoan,
CallDirect, DropValue,
Jump, BranchBool, BranchResult, Return, SimulatorFault
```

`?` baja a selección de variante, extracción y bordes de retorno/limpieza. `if` baja a ramas y unión; `while`, a condición, cuerpo y retroarista. Las llamadas usan identidades resueltas, no búsquedas de nombres durante ejecución.

### 9.2. Limpieza explícita

La IR contiene planes de limpieza por salida, compartibles mediante tablas o bloques. Cada entrada identifica el slot y su flag de vida. El intérprete no reconstruye orden léxico recorriendo el AST.

Los bordes de fallo de instrucciones y llamadas llegan a planes apropiados. La propagación de un `Err` mediante `?` usa retorno ordinario con limpieza, no el canal de `SimulatorFault`.

Los préstamos se liberan después de las acciones que los necesiten. Un plan no destruye un origen todavía prestado ni limpia un valor transferido.

No se expanden todos los planes copiando cuadráticamente listas: se permiten planes persistentes o cadenas de ámbitos identificadas, acotadas por el presupuesto de IR.

### 9.3. Verificador

Antes de producir un artefacto ejecutable se verifica:

1. Referencias y tipos válidos; argumentos y resultados coincidentes.
2. Temporales definidos en todo camino que los use.
3. Transferencias que desarman exactamente un propietario.
4. Préstamos vivos con origen vigente y permisos compatibles.
5. Ningún resultado con procedencia hacia un frame finalizado.
6. Flags y planes de limpieza coherentes en uniones y retroaristas.
7. Ausencia de AST pendiente, nodos de recuperación o características diferidas.
8. Resúmenes de efectos que cubren instrucciones, helpers y limpieza.
9. Terminadores completos y destinos existentes.
10. Origen fuente o causa sintética identificada para toda operación añadida.

Agotar análisis deja la verificación incompleta con NX-E0003; no produce un artefacto verificado ni un rechazo definitivo por tipos. Una IR inválida es un defecto interno. Las pruebas que la construyen deliberadamente se registran como pruebas del verificador, no como negativos fuente con un código `NX-E` inventado.

### 9.4. Frontera ejecutable

`CheckedProgram` se sustituye internamente por un artefacto opaco con IR verificada, perfil, manifiesto y mapa fuente. Los clientes externos no pueden fabricar su contenido.

No se expone una entrada pública del intérprete que acepte AST o IR sin verificar. Un volcado de IR es inspección, no un formato aceptado para ejecución.

La interfaz futura de backend recibe esa IR y sus contratos. Este corte no fija ABI nativa, layout portable de recursos, formato binario de módulos ni adaptadores LLVM/Cranelift; esas obligaciones D04 siguen pendientes.

## 10. Organización Rust propuesta

Se conserva inicialmente un crate y la cadena de herramientas fijada por el repositorio. No se requiere una dependencia nueva para implementar el corte.

| Área | Responsabilidad |
|---|---|
| `frontend/source.rs` | Bytes, UTF-8, spans, líneas y límites de entrada. |
| `frontend/lex.rs` | Tokens, comentarios, saltos, decimal frente a rango. |
| `frontend/parse.rs` | AST, contextos de sentencia/expresión, inserción y recuperación diagnóstica. |
| `check/names.rs` | Registro de firmas, ámbitos y resolución de intrínsecos. |
| `check/types.rs` | Tipos, contextos propietarios, constructores y firmas. |
| `check/flow.rs` | CFG de análisis, disponibilidad y punto fijo de bucles. |
| `check/loans.rs` | Procedencias, exclusividad, solapamiento y représtamos. |
| `check/effects.rs` | Conocidos/desconocidos, SCC, cadenas y comprobación de cotas. |
| `ir/model.rs` | Identidades, instrucciones, tipos y metadatos. |
| `ir/lower.rs` | Bajar control, llamadas, `?`, temporales y limpieza. |
| `ir/verify.rs` | Invariantes y recomprobación de efectos. |
| `interp/machine.rs` | Frames explícitos y ejecución de IR verificada. |
| `interp/value.rs` | Valores propietarios, copia física y destrucción iterativa. |
| `interp/mock.rs` | Manifiesto y entorno simulado configurable. |
| `interp/limits.rs` | Trabajo, frames, celdas y observaciones. |
| `diagnostic.rs` | Esquema único, catálogo, explicaciones y reparaciones. |
| `cli.rs` y `main.rs` | Lectura acotada, comandos, JSON y códigos de salida. |
| `lib.rs` | API opaca de compilación y ejecución. |

`syntax`, `check` y `runtime` actuales se migran por responsabilidades. No se mantienen dos caminos de producción con reglas distintas.


Se añaden estas responsabilidades concretas:

| Área | Responsabilidad adicional |
|---|---|
| `frontend/print.rs` | Impresor canónico del AST aceptado: terminadores explícitos, agrupación suficiente y LF. No imprime nodos de recuperación como programas válidos. |
| `interp/mock_env.rs` | Decodificación y validación acotada del esquema de §12.4; colas de respuestas y agotamiento determinista. |
| `interp/observation.rs` | Registro ordenado de observaciones, codificación de líneas, cuenta de bytes y reservas de cierre. Es compartido por ambos formatos de CLI. |
| `tests/canonical_print.rs` | Parseo–impresión–parseo, igualdad de AST normalizado e idempotencia del impresor. |
| `tests/ir_verifier.rs` | Construcción controlada y mutación de IR, únicamente para pruebas del verificador. |

El impresor es una función interna `print_canonical(&Ast) -> String`, utilizada con AST aceptados. Este corte no necesita añadir un comando público `fmt`. La normalización comparada elimina spans, comentarios y la distinción entre terminador escrito e insertado; conserva operadores, nombres, modos y estructura.

El arnés Python no recibe una entrada pública para ejecutar IR alterada. La separación de obligaciones entre Python y Rust se especifica en §15.

## 11. Límites reproducibles

### 11.1. Análisis

Las cuotas limitan el consumo del compilador. Sus unidades pertenecen al algoritmo y se versionan con el perfil; no son costes de ejecución del programa ni una obligación de simular un análisis denso cuadrático. El perfil vigente es `f0-0002-v2`.

| Presupuesto | Límite | Contabilidad y alcance |
|---|---:|---|
| `source_bytes` | 1.048.576 | Bytes del archivo antes de decodificar. |
| `syntax_nesting` | 128 | Activaciones lógicas de bloque, expresión y tipo (§13.13). |
| `expression_tree_depth` | 128 | Altura compuesta del AST (§13.13), compartida por parser y lowering. |
| `array_type_depth` | 128 | Envoltorios Array en un camino del tipo. |
| `value_type_depth` | 128 | Envoltorios Array/Result en un camino del tipo. |
| `function_count` | 1.024 | Declaraciones de función del archivo. |
| `type_solver_steps` | 1.000.000 globales | Restricciones procesadas y pares lógicos de nodos comparados. |
| `ownership_flow_steps` | 1.000.000 por función; 4.000.000 globales | Unidades dispersas definidas abajo; cada unidad carga ambos contadores. |
| `effect_fixpoint_steps` | 1.000.000 globales | Visitas a operación y propagaciones por sitio de llamada (§11.8.C). |
| `ir_items` | 262.144 globales | Registros lógicos de función, bloque, slot, instrucción, operando, préstamo y limpieza. |

Los límites globales son de una compilación completa y no se reinician entre funciones ni al verificar tras lowering. Juntos acotan el trabajo de los análisis de la compilación; no hay un presupuesto global nuevo por cada función. El contador local de propiedad empieza en cero para cada identidad de función, acumula sus pasadas y recomprobaciones, y no se reinicia al entrar en un bloque, bucle o nueva pasada de esa función. Un nuevo `check` inicia otra compilación. Los otros tres contadores costosos conservan su alcance global. El global de propiedad permite hasta cuatro presupuestos locales de trabajo concedido, para continuar después de un fallo local sin permitir que 1.024 funciones multipliquen por 1.024 el consumo total autorizado.

**Recorrido de propiedad.** Funciones en orden fuente; CFG estructurado con instrucciones en orden de evaluación. Se usa una cola de prioridad por índice de bloque, con extracción del menor índice pendiente, estabilizando localmente cada bucle antes de propagar su salida. La construcción cabecera/cuerpo/salida del CFG estructurado hace que esa prioridad sea equivalente al recorrido de postorden inverso requerido en esos CFG; no se afirma equivalencia con RPO para grafos arbitrarios. Sucesores y desempates siguen la construcción fuente, sin entradas pendientes duplicadas. Los estados son dispersos y persistentes: ausencia de hecho significa disponible, la inicialización y los permisos se verifican por sus coordenadas propias. Las uniones recorren diferencias; subárboles compartidos conservan sus transformaciones diferidas. Cada bucle indexa solo bindings leídos, consumidos, escritos o prestados por condición y cuerpo; elimina las generaciones locales al cruzar la retroarista.

**Unidad `ownership_flow_steps` de v2.** El CFG lógico previo a optimizaciones contiene construcción, instalación y limpieza de locales; no se eliminan bindings sin uso para esta cuenta. Se cobra una unidad por:

1. Instrucción examinada para construir el resumen de una retroarista, una vez por resumen; una instrucción puede pertenecer a varios resúmenes anidados.
2. Instrucción transferida en cada visita del punto fijo y en la pasada diagnóstica sobre estados convergidos.
3. Componente efectivamente unido: un subárbol presente frente a uno ausente, o compartido con sus transformaciones diferidas, cuesta una unidad sin recorrer sus hojas. Dos vacíos cuestan cero. En hojas distintas se cobra una por unir el hecho y una por cada sitio causal entrante examinado. En nodos distintos se desciende por diferencias, sin cobrar además todos los locales visibles.
4. En cabecera, hecho presente del conjunto tocado que se une; si hay dos hechos se aplica la cuenta de hoja y sitios anterior. Dos ausencias cuestan cero. No se cobran bindings no tocados como si se hubieran recorrido.

La selección de cola, referencias persistentes y desplazamientos diferidos no añaden cargos ficticios por componente visible. Cambiar unidades o secuencia de trabajo exige otro perfil; una mejora física que las conserva no. Los hijos de tipos se recorren de izquierda a derecha; efectos conserva §11.8.C. El análisis denso puede ser oráculo semántico, nunca oráculo del consumo disperso.

**Agotamiento.** Se comprueba capacidad antes de conceder cada unidad. NX-E0003 lleva `budget_kind`, `budget_scope = function | compilation`, `limit`, `observed = limit+1`, `profile_id` e `incomplete_analysis` igual a su `stage`. Propiedad local añade `analysis_function` (Entity de la función); su primario no vacío está contenido en el intervalo fuente de esa función y corresponde al cargo rechazado, nunca a otra función ni al archivo completo. En límites globales y otras guardas se omite esa clave; `budget_kind`, `budget_scope` e `incomplete_analysis` identifican el análisis incompleto sin depender del bloque donde acabó el saldo global. Si ambos límites de propiedad se excederían en el mismo cargo, se emite uno con `budget_scope = compilation`. El cargo rechazado no aumenta los contadores concedidos.

**Prefijo convergido al cortar el punto fijo.** Solo son estables los bloques alcanzables de un prefijo por índice cuyas entradas, salidas y testigos ya no pueden cambiar por ningún trabajo pendiente. Si el cargo se rechaza durante el punto fijo, se toma como frontera exclusiva `e` el mínimo entre el índice del bloque actual y los índices pendientes de la cola (si está vacía, el actual). Se retrocede `e` a la cabecera de toda retroarista con origen de índice `>= e` y destino `< e`, repitiendo hasta que ninguna la cruce. Solo los bloques de índice `< e` con estado de entrada alcanzable pertenecen al prefijo estable; haber visitado un bloque no basta. Se excluyen por completo los bucles cuya retroarista todavía puede cambiar su cabecera y los bloques posteriores dependientes de ellos. Si el punto fijo terminó, todos sus estados alcanzables están convergidos, aunque la cuota corte después durante la pasada diagnóstica. Esta definición delimita estados admisibles, no autoriza reconstruir diagnósticos mediante transferencias sin cobrar: se conservan únicamente los ya materializados desde esos estados dentro del presupuesto, con la regla siguiente.

Al agotarse una función se detiene su análisis no convergido; se conservan los diagnósticos ya obtenidos de estados convergidos, una entrada por uso. Sus códigos, subcausas, spans y testigos constituyen un subconjunto exacto de la ejecución sin cuota. Se añade NX-E0003 y se ordena por `primary_span.start` con los desempates de §13.9. Nunca se emite un diagnóstico de cabecera inestable ni se sustituye `maybe_consumed` por un estado provisional definitivo. Si la pasada diagnóstica agota la cuota tras converger, conserva lo ya materializado; no reejecuta sin límite una pasada para recuperarlo. La evidencia almacenada puede serializarse sin repetir el análisis.

Después de un límite local se analizan las funciones posteriores, en orden fuente y con cuota local nueva, dentro del saldo global. El límite global detiene el análisis restante de la compilación. Ninguno produce un artefacto aceptado como verificado ni habilita ejecución. La conservación y continuación también rigen con errores independientes en funciones anteriores. Perfiles, cuotas globales y continuación acotada se completan en I10a.

Si coinciden `array_type_depth` y `value_type_depth`, prevalece el primero. `addition_chain_length` e `index_chain_length` permanecen históricos, sin emisiones independientes. El lowering debe admitir toda altura aceptada por el parser dentro del perfil; una guarda interna no documentada no puede escapar como `OutsideScalarSlice` ni respuesta definitiva. §13.13 fija las fronteras de 127/128 términos.

`src/ir/ownership_flow.rs::Budget` y `src/ir/ownership_flow/sparse.rs` ya usan cuota local, estados persistentes, resumen por retroarista y prioridad por índice. El lowering actual (`src/ir/lower.rs`, selección de `defer_ownership`) solo invoca ese análisis para funciones que contienen `while`: 64 bindings planos no cobran hoy unidades de flujo. Aplicar la contabilidad también a funciones sin bucles, incluido el negativo `flow-32`, es obligación de I10a. Esto no acredita cuota global ni metadatos nuevos; la recuperación diagnóstica tras el límite debe cumplir la obligación acotada anterior. Las diferencias registradas en `docs/validation/F0-0002-I07b-implementation-notes.md` son obligaciones de adaptación, no conformidad retrospectiva.

### 11.2. Ejecución

| Presupuesto | Límite de `f0-0002-v2` | Fallo |
|---|---:|---|
| Trabajo | 1.000.000 unidades | `R0001` |
| Frames simultáneos de funciones Nexo, incluido `main` | 256 | `R0002`, `runtime_budget_kind = call_frames` |
| Celdas de valores vivas | 65.536 | `R0002`, `runtime_budget_kind = live_value_cells` |
| Bytes de observaciones emitidas más reservas de cierre | 65.536 | `R0002`, `runtime_budget_kind = observation_bytes` |

`runtime_budget_kind` es un campo distinto de `budget_kind`. Este último permanece reservado a la enumeración de presupuestos de análisis de `NX-E0003`.

Una celda es un escalar, unidad, recurso, cabecera de array o envoltorio de `Result`. Los payloads y elementos añaden sus celdas. Se cuentan propietarios vivos en bindings, resultados, parámetros propietarios y temporales. Un préstamo, un slot desarmado y una transferencia no añaden celdas.

Los elementos o payloads se preparan de izquierda a derecha. Al formar un agregado se añade su celda de cabecera o envoltorio y se transfieren los componentes preparados; no se vuelven a contar ni copiar. Si falla esa construcción, se limpian sus componentes inicializados. Los datos del entorno mock son plantillas del anfitrión; pasan a contar como valores Nexo al materializar una respuesta.

Trabajo:

- Una unidad por sentencia ejecutada.
- Una por nodo de expresión evaluado.
- Una por celda visitada al copiar, comparar, construir o formatear datos.
- Una por entrada de llamada, incluidas primitivas.
- Una por evaluación de condición de bucle.

Copiar una celda cobra su visita de copia una vez; instalar la celda recién copiada no añade otra visita de construcción. Construir un agregado cobra su envoltorio y las operaciones que realmente construyen o copian sus componentes. Transferir componentes ya preparados no los recorre de nuevo. Comparación y formato siguen los recorridos de §§4.6 y 12.4.

Los ticks procedentes del AST se conservan explícitamente al bajar a IR, incluido el nodo de nombre de un destino de llamada ya resuelto. No dependen del número accidental de instrucciones de Rust ni del perfil debug/release. Las primitivas no añaden frames Nexo.

Los tamaños almacenados en los valores evitan recorrer gratuitamente un árbol antes de cobrar su copia. La capacidad se comprueba antes de materializar cada resultado o copia. Los contadores usan aritmética comprobada.

Cuando una misma operación necesita varias reservas, el orden de comprobación es trabajo, frames si entra en una función, celdas y observaciones. Un fallo no ejecuta el efecto externo simulado ni instala un valor incompleto. Los argumentos ya evaluados conservan sus efectos anteriores y sus obligaciones de limpieza.

El trabajo se cobra en unidades; su primer intento excedido tiene `observed = limit + 1`. En cuotas estructurales, `observed` es la ocupación propuesta, que puede superar el límite en más de una unidad si se solicita un agregado o una línea completa. Una reserva rechazada no modifica la ocupación comprometida.

La limpieza necesaria no se corta por falta de trabajo. Antes de adquirir un recurso se reserva, dentro de observaciones, el tamaño exacto de su futuro cierre. Si no caben adquisición y cierre, se falla antes de abrirlo. La transferencia de propiedad transfiere también esa obligación; no duplica la reserva. El cierre convierte reserva en bytes emitidos.

El intérprete usa frames explícitos y recorridos acotados o iterativos. Estas cuotas limitan trabajo y estructuras del modelo, no el RSS, tiempo real, coste total de Rust ni limpieza tras aborto del proceso.

### 11.3. Alcanzabilidad de todas las cuotas de ejecución

Las siguientes son recetas constructivas del perfil normal, pendientes de ejecución. Los márgenes son cotas de diseño que el arnés debe contrastar mediante contadores; un fallo por otra guarda no satisface la fila.

| Guarda objetivo | Fuente o generador | Por qué llega antes que las otras |
|---|---|---|
| Trabajo / `R0001` | `fn main(){let r=mock_open(7);while true{}}` | Un frame, un recurso y temporales escalares de condición. Solo emite adquisición y cierre, 47 bytes en total. El bucle sigue cobrando trabajo sin aumentar las otras ocupaciones. Debe cerrar el recurso una vez. |
| `call_frames` | `fn bajar() with no_alloc,no_block,no_suspend,no_io{bajar();} fn main(){bajar();}` | El intento de frame 257 ocurre con menos de 10.000 unidades de trabajo, sin observaciones y sin crecimiento de datos. El span es la llamada que intenta entrar. |
| `live_value_cells` | `fn main(){let a0=[0];` seguido de `let aK=[aK-1,aK-1];` para `K = 1..14`, y `}`; el generador sustituye `aK-1` por el nombre anterior. | Cada valor tiene `C0=2`, `CK=1+2*C(K-1)`. Tras `a13`, los bindings suman 49.135 celdas; preparar `a14` excede 65.536. El trabajo anterior al fallo es menor de 200.000; un frame, sin observaciones, profundidad de tipo menor de 128 y fuente pequeña. |
| `observation_bytes` | `fn main(){while true{print(0);}}` | Cada salida ocupa exactamente dos bytes. Tras 32.768 líneas se han emitido 65.536 bytes; la siguiente solicita 65.538. El trabajo hasta ese intento es menor de 500.000, con un frame y un número constante de celdas. |

La receta de celdas puede incluir `let r=mock_open(7);` antes de `a0` para comprobar limpieza: añade una celda y 47 bytes comprometidos entre adquisición y cierre, sin alterar qué cuota se alcanza primero.

La prueba histórica `work_budget_exhaustion_closes_resources`, basada en duplicación de arrays, deja de ser una receta de `R0001` en este perfil. Su versión migrada de crecimiento espera `R0002/live_value_cells`, usando `runtime_budget_kind`; una regresión separada con el bucle de la primera fila comprueba `R0001` y el mismo cierre.

### 11.4. Perfiles reducidos publicados y alcanzabilidad de análisis

Cada perfil siguiente hereda íntegramente `f0-0002-v2` y cambia **solo** la cuota indicada. No cambia gramática, efectos, contabilidad ni prioridades. Los IDs son literales y forman parte del contrato de CLI y arnés.

| `profile_id` | Único cambio |
|---|---|
| `f0-0002-test-types-32-v2` | `type_solver_steps = 32`. |
| `f0-0002-test-flow-32-v2` | Límite por función de `ownership_flow_steps = 32`; global 4.000.000. |
| `f0-0002-test-flow-global-32-v2` | Límite global de `ownership_flow_steps = 32`; local 1.000.000. |
| `f0-0002-test-effects-32-v2` | `effect_fixpoint_steps = 32`. |
| `f0-0002-test-ir-32-v2` | `ir_items = 32`. |
| `f0-0002-test-frames-8-v2` | `call_frames = 8`. |
| `f0-0002-test-cells-64-v2` | `live_value_cells = 64`. |
| `f0-0002-test-observations-64-v2` | `observation_bytes = 64`. |

No se admite una opción pública para variar arbitrariamente cuotas sin un perfil publicado.

Para las cuatro guardas de análisis nuevas o costosas se exige esta cobertura fuente aislada:

| Guarda | Receta negativa en su perfil reducido | Positivo y aislamiento |
|---|---|---|
| `type_solver_steps` | `fn main(){...}` con 64 bindings distintos `let xK:i64=0;`. Cada anotación exige al menos una restricción. | `fn main(){}` completa con menos de 32 pasos. El negativo intenta el paso 33. No contiene tipos profundos, llamadas ni bucles. |
| `ownership_flow_steps` | El mismo generador de 64 bindings. Cada inicialización exige una transferencia de estado de una instrucción. | Mismo positivo; tipos y demás cuotas conservan sus límites normales. No se omiten bindings sin uso antes de esta medición. |
| `effect_fixpoint_steps` | `fn hoja(){}` y un `main` con 64 sentencias `hoja();`. Cada llamada es una operación visitada, aunque su resumen sea vacío. | `fn main(){}` queda bajo 32. No se permite acreditar esta guarda contando únicamente aristas deduplicadas entre funciones y omitiendo las visitas a las 64 operaciones. |
| `ir_items` | `fn main(){...}` con 64 bindings distintos `let xK=0;`. Cada binding requiere al menos un slot de IR. | `fn main(){}` queda bajo 32. Se cuenta la IR bajada antes de cualquier eliminación de código sin uso; tipos, flujo y efectos tienen sus presupuestos normales. |

El generador de 64 bindings se ejecuta también con `f0-0002-test-flow-global-32-v2`: exige `budget_scope=compilation`, `limit=32`, `observed=33`; el perfil local exige `budget_scope=function`, los mismos números y `analysis_function` de `main`. Ambos llevan `incomplete_analysis=ownership`. Una segunda función con un error independiente debe analizarse tras el límite local si queda saldo global. Las otras tres guardas reducidas usan `budget_scope=compilation`.

Estas fuentes son pequeñas y planas. Su implementación debe mostrar que las demás cuotas permanecen dentro de sus límites. El resultado exigido es `NX-E0003`, el `budget_kind` objetivo, el perfil publicado y `observed = 33`; no un fallo genérico.

Recetas para las otras guardas:

- `source_bytes`: fuente válida más comentario ASCII hasta alcanzar exactamente el tamaño objetivo; tamaño se comprueba antes de decodificar.
- `syntax_nesting`: bloques vacíos anidados, sin árboles profundos de expresión.
- `expression_tree_depth`: cadena aditiva construida por el parser iterativo; no se atribuye el rechazo a las guardas históricas de longitud.
- `array_type_depth`: `a0=0` y bindings sucesivos `aK=[aK-1]`; al exceder ambas profundidades se aplica la prioridad publicada.
- `value_type_depth`: comenzar por `let a0:Result<i64,i64>=Ok(0);` y envolverlo una vez por binding en 128 arrays. El último tiene 129 envoltorios de valor y solo 128 de array.
- `function_count`: 1.025 funciones vacías con nombres distintos, sin parámetros ni llamadas.

### 11.5. Fronteras y evidencia exigida

Las guardas se prueban con cantidades `N-1`, `N` y `N+1`. Un test aislado del contador no sustituye la receta fuente que demuestra alcanzabilidad.

Para ejecución hay fuentes directas:

- Frames: generar una cadena de llamadas con profundidad total 7, 8 o 9, incluido `main`, bajo el perfil de ocho frames.
- Celdas: `fn main(){let a=[...];drop(a);}` con 62, 63 o 64 literales `0`, bajo el perfil de 64 celdas. La cabecera añade una celda: se solicitan 63, 64 o 65. `drop(a)` libera el array antes de construir el resultado unidad final.
- Observaciones sin recursos, bajo el perfil de 64 bytes: 30 llamadas `print(0);` y una `print(10);` solicitan 63 bytes; 32 llamadas `print(0);`, 64; 31 llamadas `print(0);` y una `print(10);`, 65.
- Observaciones con reserva: `mock_open(7)` y su cierre comprometen 47 bytes. Ocho `print(0)` dan 63; siete `print(0)` y un `print(10)` dan 64; nueve `print(0)` intentan 65. En el último caso se rechaza la novena salida y el cierre reservado deja 63 bytes efectivamente emitidos.
- Trabajo: el modelo independiente calcula los ticks de un bucle finito de incremento y añade bloques vacíos independientes, de un tick cada uno, para completar `N-1`, `N` y `N+1`. La fuente generada es corta; no se rellena un archivo de más de un MiB.

Para los cuatro contadores de análisis reducidos se prueban además prefijos de 31, 32 y 33 cargos en su adaptador de presupuesto. Las fuentes de §11.4 deben alcanzar el cargo 33 dentro del análisis real. Ambos resultados se presentan separados: frontera del contador y alcanzabilidad desde fuente.

Todo informe conserva perfil, fuente o generador, código, etapa, campo de presupuesto correcto, límite, ocupación propuesta, span y limpieza. La comparación del span aplica §13.13: propiedad de validez y no vaciedad para los cuatro presupuestos costosos de análisis; igualdad exacta para los demás. La coincidencia del código sin esa evidencia no acredita la guarda.


### 11.6. Contabilidad normativa de trabajo

Esta subsección concreta exhaustivamente las categorías de §11.2. La unidad es un **tick lógico del modelo**, independiente de cómo Rust represente el AST, la IR, un escalar o un préstamo.

Se conserva una secuencia ordenada de cargos. Las optimizaciones, el interning y la agrupación de instrucciones no pueden cambiar esa secuencia observable, su total ni el span del primer cargo rechazado.

#### Contextos y materialización

Se distinguen tres operaciones:

- **Lectura:** observar un valor existente sin crear un propietario. Leer un escalar en un registro de operación no crea una celda Nexo y no cobra una copia. El valor escalar izquierdo queda determinado antes de evaluar el operando derecho.
- **Copia propietaria:** crear un valor independiente a partir de un lugar. Cobra una vez por cada celda copiada, también si el valor es un único `i64`, `bool` o unidad.
- **Transferencia:** cambiar el propietario de un valor existente. No recorre ni copia sus celdas y no cobra construcción.

Los operandos de `+`, `-`, `<` y `==`, las condiciones, los índices y los argumentos ordinarios se evalúan para lectura. Los inicializadores, RHS, elementos de array, payloads de `Ok`/`Err`, valores de rama, retornos y argumentos propietarios requieren propiedad.

Una expresión que ya produce un temporal propietario lo entrega sin una segunda copia. Un nombre o una proyección sobre un propietario existente, solicitado en contexto propietario, sí exige la copia correspondiente, salvo transferencia explícita.

Los paréntesis de agrupación no crean nodos cobrables. Los nombres que forman un `Place` de destino o de acceso explícito no se reinterpretan además como expresiones de lectura. Esta distinción se aplica antes del lowering.

#### Tabla de construcciones sintácticas

Los cargos de la columna central son adicionales a los de sus hijos y a las operaciones de la tabla siguiente. Se cobran antes de evaluar los hijos, salvo indicación expresa.

| Construcción | Ticks propios | Evaluación y precisión |
|---|---:|---|
| `Program`, declaración `Function`, firmas, parámetros, tipos, anotaciones y `WithClause` | 0 | No se ejecutan como sentencias. |
| Entrada inicial a `main` | 1 | Entrada de función, sin inventar una expresión de llamada ni un nombre de callee escrito. |
| `Block` usado como cuerpo de función, rama o bucle | 0 | Sus llaves son contenedor; se cobran sus sentencias. |
| `Block` usado como sentencia independiente, incluido `{}` | 1 | Se cobra una vez al entrar. |
| `Binding` | 1 | Evalúa el inicializador en contexto propietario; instalar el binding cuesta 0. |
| `Assignment` | 1 | Evalúa el destino como `Place`, sus índices y después el RHS propietario. La instalación y liberación del valor sustituido cuestan 0. |
| `ReturnStmt` | 1 | Evalúa el operando propietario, si existe. Un retorno vacío construye unidad, sin nodo de expresión adicional. |
| `ValueStmt` | 1 | Evalúa su operando propietario. `value:` no añade un nodo de nombre ni una llamada. |
| `ExprStmt` | 1 | Evalúa su expresión. Descartar el resultado cuesta 0. |
| `IfForm` en posición de sentencia | 1 | Es el cargo de la sentencia; no se añade otro por un `if` expresión. Evalúa condición y únicamente la rama seleccionada. |
| `IfForm` en posición de expresión | 1 | Evalúa condición y rama seleccionada; recibe por transferencia el valor de `value:`. |
| `WhileStmt` | 1 al entrar | Además cobra 1 antes de **cada** evaluación de condición, incluida la última falsa. Su cuerpo contenedor cuesta 0. |
| Literal entero, booleano o unidad `()` | 1 | Además construye una celda. Un entero firmado es un único literal, no un nodo unario adicional. |
| Nombre evaluado como expresión | 1 | En lectura no copia; en contexto propietario añade la copia por celdas. |
| Nombre del destino de una llamada directa | 1 | No materializa un valor función ni cobra copia. |
| Expresión de array | 1 | Evalúa elementos propietarios de izquierda a derecha y después construye su cabecera. |
| Operador binario `+`, `-`, `<` o `==` | 1 | Evalúa ambos operandos para lectura, de izquierda a derecha; los costes propios de la operación figuran abajo. |
| Expresión indexada `base[indice]` | 1 | Evalúa base para lectura e índice para lectura; no copia la base. Véase extracción abajo. |
| `PlaceRoot` de asignación, `consume`, `inout` o argumento lugar de `drop` | 0 | Resuelve una identidad y no constituye una expresión de nombre adicional. |
| Cada selector `[indice]` dentro de un `Place` | 1 | Cobra antes de evaluar su índice. Las comprobaciones de límites del destino de asignación siguen ocurriendo después del RHS. |
| Prefijo `consume` sobre un binding completo | 1 | Después resuelve su `Place`; transferir las celdas cuesta 0. |
| Prefijo `inout` sobre un lugar | 1 | Después evalúa su `Place`; adquirir y terminar el permiso cuestan 0. |
| Llamada directa, incluidos intrínsecos y constructores `Ok`/`Err` | 1 | Después cobra el nombre del callee, evalúa argumentos en orden escrito y cobra 1 de entrada de llamada si todos quedaron preparados. |
| Operador `?` | 1 | Evalúa su operando propietario. Inspección de etiqueta y transferencia se precisan abajo. No añade una llamada. |
| `Expr`, `CompareExpr`, `AddExpr`, `TryExpr`, `AccessExpr`, `Postfix` sin un nodo concreto de las filas anteriores | 0 | Son producciones envolventes; no acumulan cargos por atravesar niveles de precedencia. |
| `Argument`, etiqueta de argumento, listas, comas y separadores | 0 | Solo se cobran las expresiones contenidas. |
| Agrupación `(e)` o `(Place)` | 0 | Conserva el contexto del hijo. No es el literal unidad `()`. |
| `Terminator`, `Name` de declaración y `ValueName` sintáctico | 0 | No se ejecutan. |
| Formas rechazadas o nodos de recuperación | No ejecutables | Nunca llegan a un artefacto ejecutable. |

Las llamadas a funciones Nexo añaden un frame; las primitivas y `Ok`/`Err` cobran entrada de llamada, pero no añaden frames Nexo.

Un argumento lugar de `drop` se evalúa como `Place`; `drop(x)` no cobra además una expresión de nombre `x`. Un argumento temporal, por ejemplo `drop(Ok(1))` con contexto de tipo suficiente, conserva todos los cargos de la expresión que lo construye.

#### Tabla de operaciones

| Operación | Cargo y orden |
|---|---|
| Construcción de un escalar, unidad, recurso, cabecera de array o envoltorio de `Result` | 1 por celda nueva. Se cobra antes de materializarla. |
| Copia propietaria | 1 por celda copiada. Incluye cabeceras y envoltorios, y únicamente el payload activo de un valor `Result`. No añade un segundo cargo de construcción por esas mismas celdas. |
| Transferencia, instalación en slot, asociación a parámetro, entrega de retorno o de rama | 0. |
| Lectura de escalar existente | 0 adicional al nodo que solicita la lectura. No es una copia propietaria. |
| Suma o resta comprobada | El cálculo queda cubierto por el tick del operador. Si tiene éxito, añade 1 por la celda del resultado. Si desborda, produce `R0101` o `R0103` sin construir resultado. |
| `<` sobre `i64` | Después de evaluar operandos, 2: visita al escalar izquierdo y al derecho. Después añade 1 por el booleano resultado. |
| `==` y comparación de `assert_eq` | 2 por cada par de celdas visitado, izquierda antes de derecha. Arrays: cabeceras, longitud y luego elementos en orden creciente. Result: envoltorios, etiqueta y luego payload si coincide. Se detiene en la primera diferencia. |
| `==` satisfactorio como operación | Después del recorrido, 1 por el booleano, tanto si es `true` como `false`. |
| `assert_eq` | Añade el recorrido de comparación y, solo si hay igualdad, 1 por su resultado unidad. Si no, produce `R0201` sin construir unidad. |
| Indexación de un lugar | Las comprobaciones de índice y la selección no añaden ticks. Si se pide propiedad, copia únicamente el elemento seleccionado; si se pide lectura, no copia. |
| Indexación de un temporal propietario | Transfiere el elemento seleccionado sin copia y limpia el resto. No vuelve a cobrar sus celdas. |
| `?` con `Ok` | Retira el envoltorio y transfiere el payload, sin ticks adicionales. |
| `?` con `Err` | Transfiere el error hacia el retorno y reutiliza su celda de envoltorio para el tipo de resultado de la función. No construye otro wrapper, no ejecuta un `ReturnStmt` sintético ni crea unidad. La igualdad del tipo de error ya está comprobada. |
| `Ok(value)` / `Err(value)` intrínsecos | Tras sus cargos de llamada y el payload propietario, 1 por el wrapper. Transferir el payload al wrapper cuesta 0. |
| Retorno implícito de una función unidad que alcanza su cierre | 1 por construir unidad; no hay sentencia `return` ni expresión `()` sintéticas cobrables. |
| `len` | 1 por construir su resultado `i64`. Leer la longitud almacenada cuesta 0. |
| `try_add` | 2 por construir el escalar de éxito o error y su wrapper. No ejecuta un nodo fuente `+` ni produce `R0101`. |
| `print` | 1 por cada celda del dato formateado, en preorden y con elementos de izquierda a derecha; después 1 por el resultado unidad. Caracteres, comas, paréntesis y bytes de transporte no añaden ticks de trabajo. |
| `mock_open` / `mock_blocking_open` | 1 por la celda de recurso. La etiqueta ya fue evaluada como argumento. |
| `inspect` / `mock_unknown` | 1 por su resultado unidad. |
| `mock_read_records` | Materialización de la respuesta: `2 + 2*n` celdas para `Ok` de `n` registros; 2 para `Err`. Cada registro tiene wrapper y escalar. |
| `mock_write_i64` | 2 por el `Result<(),i64>` devuelto; si tiene éxito, además 1 por formatear el entero de la línea de salida. En error no formatea una salida inexistente. |
| `mock_request` / `mock_reply` | 2 por el resultado, tanto en éxito como en error. |
| `drop` | La destrucción cuesta 0; añade 1 por su resultado unidad. |
| `fail` | Después de la entrada de llamada produce `R0301`; no construye resultado. |
| Creación, congelación, restauración y fin de préstamos; comprobaciones de permiso | 0 durante ejecución. |
| Destrucción explícita o implícita, liberación de temporales y cierre reservado de recursos | 0; no puede interrumpirse por agotamiento de trabajo. |
| Formato de eventos mock, diagnósticos, informe final y salida de `explain` | 0. Los eventos formatean metadatos del simulador; sus bytes siguen contando en observaciones cuando corresponde. |

Los cargos de una primitiva necesarios para su resultado y salida se completan **antes** de comprometer sus efectos. En particular, `drop` cobra por adelantado la unidad resultante; después destruye y materializa esa unidad. Su comprobación de celdas tiene en cuenta ese orden: no mantiene simultáneamente el valor destruido y una unidad todavía no creada. Esto conserva la frontera de 64 celdas de §11.5.

Fuera de esa destrucción explícita, no se adelanta la liberación de argumentos prestados ni de locales para aparentar una ocupación menor. El resultado de retorno sigue preparándose antes de limpiar el frame.

#### Cargos agrupados y `observed`

El contrato de trabajo es unitario, aunque una implementación pueda agrupar una secuencia de ticks del mismo origen para comprobar capacidad antes de copiar:

```text
charge(k, origin):
    remaining = limit - used
    if k <= remaining:
        used = used + k
        continuar
    else:
        used = limit
        producir R0001 con:
            limit = limit
            observed = limit + 1
            work_consumed = limit
            primary_span = origin
```

`k` es no negativo. No se calcula `used + k` con aritmética susceptible de desbordamiento para decidir el fallo.

En el segundo brazo se concedió conceptualmente el prefijo de ticks que cabía, pero **no se materializa parcialmente la copia ni se compromete la primitiva**. `observed` identifica el primer tick rechazado, no la suma de todo el lote solicitado. Por ejemplo, con límite 10, consumo previo 9 y copia de cinco celdas, se informa `observed = 11`, `work_consumed = 10`, no 14.

Las reservas de frames, celdas y observaciones siguen otra regla: son estructurales y atómicas; su `observed` es la ocupación propuesta completa. No se aplica a R0002 la saturación de trabajo.

No se agrupan cargos de distintos orígenes si ello cambia qué span corresponde al primer tick rechazado.

#### Origen de los ticks

- Sentencia con introductor: token `let`, `var`, `return`, `while`, `if` o rango `value:`. Bloque independiente: su `{`.
- Asignación: lugar destino completo. Sentencia de expresión: expresión completa, sin terminador.
- Nodo de expresión: su construcción fuente completa; nombre: su lexema; índice de `Place`: selector completo `[indice]`.
- Prefijo: token `consume` o `inout`.
- Entrada de llamada: llamada completa. Entrada inicial: nombre `main`.
- Tick adicional de condición de bucle: expresión de condición.
- Copia, comparación, formato o construcción: expresión o llamada que solicita esa operación.
- Unidad de `return;`: token `return`. Unidad implícita: `}` del cuerpo de la función.

Las agrupaciones conservan spans fuente aunque no tengan tick propio. Un operador binario usa el span completo de la expresión para sus ticks de operación y resultado.

#### Cambio explícito respecto de F0-0001

`src/runtime.rs::data` copiaba y cobraba todas las celdas de un binding, incluido un escalar leído como operando de `+`. F0-0002 cambia esa conducta: en contexto de lectura no hay copia propietaria ni ese cargo por celda. El tick del nombre permanece.

También se añaden cargos que F0-0001 no tenía: construcción de celdas, comparación, formato, entrada de llamadas, condiciones de bucle y resultados unidad. La indexación deja de copiar toda la base. Los cargos especiales de `move` y `mock_open` del runtime anterior no se trasladan como excepciones implícitas.

Por ello no se promete conservar los totales de trabajo de F0-0001. Se conservan sus resultados históricos y se publican las nuevas cuentas bajo el perfil de este ADR.

### 11.7. Totales numéricos y fronteras exactas de R0001

Los cálculos siguientes son expectativas del modelo, no resultados de ejecutar una implementación Nexo.

#### Cargos elementales y unidad final

Se conserva el modelo lógico de §11.6: no se cobran instrucciones administrativas añadidas por lowering. Construir la unidad final sí crea una celda; cada selector del destino sí es trabajo distinto del índice y del RHS.

| Fuente u operación | Desglose normativo | Ticks |
|---|---|---:|
| `fn main(){}` completo | Entrada 1 + unidad final 1; cuerpo contenedor 0 | 2 |
| `a[1]=5;`, con `a` ya preparado | Asignación 1 + selector 1 + índice literal 2 + RHS literal 2 | 6 |
| `{}` como sentencia | Entrada al bloque independiente | 1 |

Los 3 ticks de la función vacía frente a 2, los 5 de la asignación indexada frente a 6, los 67 del bucle finito de abajo frente a 56 y los 0 del bloque independiente `{}` frente a 1 son diferencias de la implementación a corregir en I10b, no nuevos esperados. `src/ir/lower.rs` emite la unidad de cierre como instrucción con tick; las notas I05/I06 registran el cargo indexado incompleto y el origen del cuerpo en lugar del cierre. La tabla anterior decide el modelo, independientemente de esos detalles internos. El span de R0001 o R0002 al construir unidad final es exclusivamente la última `}` del cuerpo de esa función; en `fn main(){}` es `[10,11)`. La limpieza conserva §6.4 sin cargos de trabajo. La familia exacta de abajo alcanza esa unidad final sin calibración contra el intérprete.

#### Bucle finito

```nexo
fn main(){var i=0;while i<3{i=i+1;}}
```

| Parte | Cálculo | Ticks |
|---|---|---:|
| Entrada a `main` | 1 | 1 |
| `var i=0;` | Binding + nodo entero + celda | 3 |
| Entrada a `while` | 1 | 1 |
| Cada condición `i<3` | Tick de condición + nodo `<` + nombre `i` + nodo y celda de `3` + dos visitas + celda booleana | 8 |
| Cuatro condiciones | `4*8` | 32 |
| Cada `i=i+1;` | Asignación + nodo `+` + lectura de `i` + nodo y celda de `1` + celda de suma; Place raíz 0 | 6 |
| Tres asignaciones | `3*6` | 18 |
| Unidad implícita final | 1 | 1 |
| **Total** | `1+3+1+32+18+1` | **56** |

Para `k >= 0`, sustituyendo el límite literal por `k`, el total es:

```text
W(k) = 1 + 3 + 1 + 8*(k+1) + 6*k + 1
     = 14*k + 14
```

La anchura decimal del literal no modifica el trabajo.

#### Llamada con Result y propagación exitosa

```nexo
fn inc(n:i64)->Result<i64,i64>{return Ok(n+1);}
fn main()->Result<(),i64>{let x=inc(2)?;return Ok(());}
```

| Parte | Ticks |
|---|---:|
| Evaluar `n+1`: operador 1, nombre leído 1, literal 2, resultado 1 | 5 |
| `Ok(n+1)`: llamada 1, nombre 1, payload 5, entrada 1, wrapper 1 | 9 |
| Cuerpo de `inc`, incluido `return` | 10 |
| `inc(2)`: llamada 1, nombre 1, argumento 2, entrada 1, cuerpo 10 | 15 |
| `inc(2)?` | 16 |
| Binding `let x=...;` | 17 |
| `return Ok(());`: return 1, llamada 1, nombre 1, unidad literal 2, entrada 1, wrapper 1 | 7 |
| Entrada inicial a `main` | 1 |
| **Total** | **25** |

Si el cuerpo de `inc` se sustituye por `return Err(9);`, su cuerpo cuesta 7, el binding de `main` alcanza la propagación con coste 14 y la ejecución completa cuesta **15** incluida la entrada inicial. El `return Ok(());` de `main` no se ejecuta y no se cobra un retorno sintético por `?`.

#### Familia exacta para 999.999, 1.000.000 y 1.000.001

Generar, sin saltos finales necesarios:

```text
"fn main(){var i=0;while i<71427{i=i+1;}"
+ "{}" repetido p veces
+ "}"
```

El bucle y la salida implícita cuestan `14*71427+14 = 999992`. Cada bloque independiente añade exactamente un tick.

| `p` | Trabajo total solicitado | Resultado con límite 1.000.000 |
|---:|---:|---|
| 7 | 999.999 | Termina normalmente. |
| 8 | 1.000.000 | Termina normalmente. |
| 9 | 1.000.001 | `R0001`, `observed = 1000001`, `work_consumed = 1000000`. |

En el último caso, el tick rechazado es la construcción de unidad implícita, sobre la última `}`. La fuente es corta, usa un frame, un número constante de celdas y ninguna observación. Esta familia concreta la receta de trabajo de §11.5 sin calibrarse contra el compilador bajo prueba.

### 11.8. Alcanzabilidad de los cuatro presupuestos de análisis en el perfil normal

Los perfiles reducidos siguen siendo obligatorios, pero no sustituyen estas familias del perfil **normal** `f0-0002-v2`.

Para que sus cálculos sean reproducibles se fijan estas precisiones de contabilidad:

1. Una comparación estructural de tipos cobra cada par lógico de nodos visitado, incluso cuando los tipos comparten identidad internada. Una caché puede ahorrar CPU, pero no el cargo lógico publicado. La comparación de dos tipos iguales `Array` de profundidad 64 sobre `i64` visita 65 pares.
2. Propiedad utiliza las unidades dispersas de §11.1. No cobra componentes visibles no recorridos. Los bordes terminales no crean uniones normales ficticias; limpieza usa planes persistentes y flags.
3. El punto fijo de efectos de una SCC utiliza rondas síncronas: la ronda lee los resúmenes de la ronda anterior y produce los siguientes. Los resúmenes iniciales contienen las contribuciones locales conocidas/desconocidas. Se recorren funciones y sitios de llamada en orden fuente; una cola FIFO por ronda no contiene duplicados. Se cobra cada visita local a operación y cada propagación por sitio de llamada de cada ronda, incluida la ronda que demuestra estabilidad. No se deduplican sitios distintos por tener el mismo callee.
4. `ir_items` cuenta registros lógicos, no únicamente objetos asignados por Rust. Cada ocurrencia de operando cuenta, incluido destino y literal, aunque esté codificada como campo inline o comparta una constante internada. Un binding escalar directo necesita, como mínimo, slot, instrucción `Const`, operando literal y operando destino: cuatro ítems.
5. Los contadores globales no se reinician entre funciones ni al recomprobar. El local de propiedad es por identidad de función (§11.1). Las familias distinguen aceptación, límite local y global; ninguna cuenta densa sustituye sus unidades.

Los generadores siguientes producen ASCII, sin comentarios ni espacios adicionales y sin LF final. Los tamaños indicados se obtienen de esas cadenas exactas.

#### A. `type_solver_steps`

Generar:

```text
"fn main(){let a0=0;"
+ para j=0..63: "let a" + (j+1) + "=[a" + j + "];"
+ "var y=a64;"
+ "y=a64;" repetido 15400 veces
+ "}"
```

Tamaño: **93.307 bytes**.

Cada asignación compara el tipo esperado de `y` con el de `a64`; ambos tienen 64 envoltorios Array. Solo los pares de nodos de esas comparaciones ya cuestan:

```text
15400 * 65 = 1001000 > 1000000
```

Los cargos de restricciones e inferencia inicial adelantan el fallo, no lo evitan. La profundidad de tipo es 64; los AST de cada asignación son pequeños. El límite se alcanza durante tipos, antes de análisis de propiedad, efectos o construcción de IR, y sin exceder fuente, funciones ni anidamiento.

#### B. `ownership_flow_steps`

Estas fuentes son para `check`, incluso si sus bucles no terminarían en ejecución. Son ASCII sin LF final; grafías y tamaños están sujetos a F0-D15. Los resultados no se calibran ejecutando el compilador probado.

**B-A1, uniones sin diferencias.** Se conserva la fuente de 23.491 bytes:

```text
"fn f(b:bool){"
+ para j=0..1023: "let x" + j + "=0;"
+ "if b{}else{}" repetido 1024 veces
+ "}fn main(){}"
```

Esperado: aceptación, `outcome.kind=checked`, salida 0, `diagnostics=[]`. Los estados sin consumos no unen 1.025 componentes por rama; se retira el agotamiento basado en `1024*1025`.

**Familia N(nombre,k), resúmenes de bucles anidados.**

```text
"fn " + nombre + "(b:bool){"
+ "while b{" repetido 96 veces
+ para j=0..k-1: "let x" + j + "=0;"
+ "}" repetido 97 veces
```

No hay consumos ni cambios de hechos en retroaristas; cada cabecera converge sin repropagar un estado cambiado. Cada binding interior tiene construcción, instalación y limpieza: al menos tres instrucciones de propiedad. Los 96 resúmenes examinan esas instrucciones. Para estas instancias escalares, el CFG lógico usa como máximo ocho instrucciones por binding y menos de 20.000 unidades de infraestructura de bucles, incluyendo resúmenes, transferencias y diagnóstico. Así, k=1.024 queda bajo `98*8*1024+20000=822816`, sin uniones de hechos presentes. Es una cota para este generador, no para cualquier fuente del mismo tamaño.

| ID | Fuente generada | Bytes | Esperado literal |
|---|---|---:|---|
| B-A2 | `N("f",64)+"fn main(){}"` | 1.519 | `checked`, salida 0, `diagnostics=[]`. |
| B-A3 | `N("f",256)+"fn main(){}"` | 3.595 | `checked`, salida 0, `diagnostics=[]`. |
| B-A4 | `N("f",1024)+"fn main(){}"` | 12.067 | `checked`, salida 0, `diagnostics=[]`. |
| B-L1 | `N("f",6000)+"fn main(){}"` | 71.779 | Una NX-E0003, `stage=ownership`, `operation=analysis.limit`, `budget_kind=ownership_flow_steps`, `budget_scope=function`, `limit=1000000`, `observed=1000001`, `incomplete_analysis=ownership`, `analysis_function={"kind":"function","id":"function:3"}`; `rejected`, salida 1. |
| B-G1 | Concatenar `N("f"+j,1024)` para j=0..13, después `"fn main(){}"` | 168.813 | Una NX-E0003, `stage=ownership`, `operation=analysis.limit`, `budget_kind=ownership_flow_steps`, `budget_scope=compilation`, `limit=4000000`, `observed=4000001`, `incomplete_analysis=ownership`; `rejected`, salida 1. `analysis_function` ausente. |

B-L1 excede la cuota local solo con `96*3*6000=1728000` visitas de resumen. B-G1 contiene 14 funciones N más `main`: excede la global solo con `14*96*3*1024=4128768 > 4000000`, aunque cada función queda bajo su cuota local. Con 13 funciones esa cota sería `3833856 < 4000000`. Tipos escalares, 96 bucles dentro de `syntax_nesting`, sin efectos transitivos ni errores de programa. El span del límite cumple §13.13, sin offset obligatorio.

**Orden normativo del arnés de §11.8.B.** Primero se baja toda la IR construible de las funciones del archivo y se acumula `ir_items`, incluidos los registros parciales de una función rechazada durante su construcción. Solo después se ejecuta el análisis de propiedad sobre las funciones construidas, en orden fuente, con los contadores local y global de §11.1; no se intercala bajada y análisis función a función. Una función parcialmente construida conserva su diagnóstico independiente y no se presenta como IR completa ni verificada. El arnés debe informar el total acumulado de `ir_items` del archivo, no un máximo por función ni solo la IR de funciones aceptadas, además de los contadores de las otras etapas. Debe demostrar que ninguna otra cuota, incluida `ir_items`, impide alcanzar el límite de propiedad. Este orden físico de construcción para el arnés es distinto del orden de barreras semánticas de §13.9; no adelanta la verificación final ni permite ejecutar IR incompleta. La implementación actual intercala ambos trabajos y carece de cuota global de propiedad; esta obligación se cierra en I10a.

**Convención reproducible de las mediciones de IR de B-G1/B-D1.** Se instrumentan los registros de `src/ir/model.rs` construidos por `src/ir/lower.rs` y los cargos de `src/ir/ownership_flow.rs::Budget::charge`, conservando el recorrido de `src/ir/ownership_flow/sparse.rs::analyze`. Se cuenta un registro por función, bloque, slot local (incluidos parámetros), instrucción, terminador, préstamo creado y entrada de limpieza. Cada `output=Some` suma un operando destino; `None` no suma. Los temporales no añaden un slot local aparte. Los operandos de entrada se cuentan por ocurrencia con la tabla siguiente; sus identidades internadas o campos inline no eliminan el cargo. Los tipos, spans, permisos y etiquetas de opcode/intrínseco son metadatos, no operandos adicionales. La lista de limpieza cuenta una entrada por local en estas mediciones; no vuelve a contar su LocalId como operando. Es la convención de medición de esta IR concreta conforme al punto 4 de §11.8, no un cambio de límite ni una garantía de representación física.

| Operación presente en estas fuentes | Operandos de entrada; añadir el destino si existe |
|---|---:|
| `ConstI64`, `ConstBool`, `ConstUnit` | 1 literal, incluida la unidad. |
| `CopyValue`, `BorrowValue`, `DropBinding`, `DropLocal`, `DropValue` | 1 local o temporal. |
| `WritePlace` | 2: local y valor. |
| `CallIntrinsic` | Un operando por argumento; el selector de intrínseco no suma. |
| `Jump` | 1 destino de bloque. |
| `BranchBool` | 3: condición y dos destinos de bloque. |
| `Return` | 1 si lleva valor; 0 si no lo lleva. |

Medición de B-G1 sobre una copia instrumentada del código actual, con toda la IR construida antes de propiedad: **168.813 bytes**, **11.593 ítems por función N** = 1 función + 289 bloques + 1.025 slots + 3.458 instrucciones/terminadores + 5.795 operandos + 0 préstamos + 1.025 entradas de limpieza; `main` añade **7** (1 función, 1 bloque, 2 instrucciones/terminadores y 3 operandos). Total: **162.309 ítems**, menor que **262.144**. Con las guardas de propiedad elevadas únicamente para medir el recorrido completo, cada N cobra **305.906 unidades**, `main` cobra **2** y el archivo **4.282.686**. Con 13 funciones N más `main` serían **3.976.780** unidades, por debajo de la global. Con límites normales en el arnés, las primeras 13 N conceden **3.976.778** unidades; `f13` concede **23.222** y rechaza el cargo global **4.000.001**, primario medido `[166193,166194)`. El rechazo es global de propiedad: ninguna función llega a 1.000.000 unidades locales y `ir_items` ya quedó en 162.309. No se analiza `main` tras ese rechazo. La instrumentación de la cuota global demuestra el aislamiento de la fuente; no acredita su implementación ni los metadatos v2 en el compilador actual.

**B-D1, conservación y continuación.**

```text
"fn big(b:bool){var s=mock_open(1);drop(s);while b{inspect(s);s=mock_open(2);}"
+ "while b{" repetido 96 veces
+ para j=0..5999: "let x"+j+"=0;"
+ "}" repetido 97 veces
+ "fn main(){}"
```

Tamaño **71.843 bytes**. La función `big` termina en 71.832; el sufijo anidado conserva la familia N. Sus resúmenes bastan para exceder propiedad (`96*3*6000=1728000`) sin la IR de 13.000 bucles planos. La fuente anterior de 824.759 bytes (824.796 con `later`) queda sustituida: su tamaño era correcto, pero excedía `ir_items` antes de aislar propiedad.

Medición sobre una copia del código actual con toda la IR construible bajada antes del análisis de propiedad y la convención anterior: `big` tiene **61.400 ítems** = 1 función + 292 bloques + 6.002 slots locales + 18.401 instrucciones (incluidos terminadores) + 30.702 ocurrencias de operandos + 0 préstamos + 6.002 entradas de limpieza. `main` añade 7: **61.407 ítems** construidos en la compilación, menor que 262.144. El primer cargo rechazado de propiedad es **1.000.001**, primario medido `[49737,49738)`. Con las guardas elevadas únicamente para medir el recorrido completo, `big` necesita **1.768.894 unidades** y `main` **2**, total **1.768.896**; no cambia el límite normativo. La variante `later` conserva esos 61.407 ítems completos y añade **14 ítems parciales medidos**: 1 función y 1 bloque reservados, 1 slot, 4 instrucciones y 7 operandos; total **61.421**, también bajo 262.144. Su error secuencial se descubre antes de finalizar su IR y crear su lista de limpieza; no se cuenta esa función como si no hubiera construido nada. En ambas variantes con cuota local normal, `big` recibe 1.000.000 unidades antes del rechazo y `main` añade 2, sin agotar el saldo global. La contabilidad global y metadatos v2 siguen pendientes de I10a; estos datos medidos no los acreditan.

Esperado normativo, lista en este orden, `outcome.kind=rejected`, salida 1:

1. NX-E0202/`maybe_consumed`, `stage=ownership`, `operation=ownership.use`, primario `[58,59)`, `data={}`. Relaciones `declaration=[19,20)` y `consumption=[34,41)`; cadena única `kind=drop`, `operation=value.drop`, span `[34,41)`, Entity `{"kind":"binding","id":"binding:19"}`, `evidence=verified`. El testigo llega del consumo previo al bucle; la retroarista reinicializa `s`, pero no integra esa cadena. Es idéntico al de la fuente corta que omite el sufijo anidado.
2. NX-E0003, `subcause=null`, `stage=ownership`, `operation=analysis.limit`, `profile_id=f0-0002-v2`, `data={"budget_kind":"ownership_flow_steps","budget_scope":"function","limit":1000000,"observed":1000001,"incomplete_analysis":"ownership","analysis_function":{"kind":"function","id":"function:3"}}`. Relaciones y cadena `[]`. Conforme a §13.13, el primario medido `[49737,49738)` no es un offset portable exigido: debe ser no vacío, válido y estar dentro del sufijo anidado de `analysis_function`, después del uso `[58,59)` y antes del cierre de `big` en `[71831,71832)`. Esta propiedad adicional hace obligatorio el orden de la lista; un primario `[3,6)` no satisface B-D1.

Añadir antes de `fn main(){}` la función `fn later(){let z=0;drop(z);print(z);}` produce **71.880 bytes** y exige una tercera entrada NX-E0202/`definitely_consumed`, `stage=ownership`, `operation=ownership.use`, `data={}`. El límite local no oculta la función posterior. La lista se compara ordenada, una entrada por uso, sin estados provisionales duplicados.

| Dato del diagnóstico posterior | Literal |
|---|---|
| Primario | `[71865,71866)` (`z` de `print`). |
| Relación `declaration` | `[71847,71848)`. |
| Relación `consumption` | `[71851,71858)` (`drop(z)`). |
| Cadena | Un paso `kind=drop`, `operation=value.drop`, span `[71851,71858)`, Entity `{"kind":"binding","id":"binding:71847"}`, `evidence=verified`. |

Las entradas NX-E0202 tienen `profile_id=f0-0002-v2`; ambas reglas y explanation_id son código/subcausa. NX-E0003 usa regla y explanation_id `NX-E0003`. En las tres, `feature_id`, token, parser_context, recognized_prefix y expected_tokens_kind son null; expected_tokens y repair son `[]`, namespace `Nexo-proposed-v1`, severity error. La implementación actual conserva los errores y el límite medidos, pero aún omite budget_scope, incomplete_analysis y analysis_function; no se transcriben esas omisiones como expectativas.

Los perfiles reducidos local/global y las fronteras de contador complementan estas familias. No se promete aceptar cualquier programa válido de 65 KiB: se exige aceptar B-A1–B-A4 y rechazar controladamente B-L1/B-G1/B-D1. Los bytes por sí solos no acotan resúmenes e iteraciones.

#### C. `effect_fixpoint_steps`

Generar 1.022 funciones formando un anillo:

```text
para j=0..1021:
    "fn f" + j + "(){f" + ((j+1) mod 1022) + "();"
    + ("print(0);" si j=1021; cadena vacía en otro caso)
    + "}"
+ "fn main(){}"
```

Tamaño: **18.240 bytes**; funciones: **1.023**, incluida `main`.

Solo `f1021` tiene inicialmente el efecto local `io`. Con rondas síncronas, el efecto avanza un borde del anillo por ronda. Se necesitan 1.021 rondas para que alcance el último nodo restante, antes de la ronda final de estabilidad.

Solo las propagaciones de los 1.022 sitios del anillo durante esas rondas cuestan:

```text
1022 * 1021 = 1043462 > 1000000
```

Las visitas locales añaden trabajo. Tipos y propiedad analizan cuerpos pequeños, sin joins, recursos ni parámetros; quedan por debajo de 100.000 pasos. Se comprueban todas las funciones aunque `main` no las llame. No se ejecuta la recursión: esta es una fixture de `check`.

#### D. `ir_items`

Generar:

```text
"fn main(){"
+ para j=0..65536: "let x" + j + "=0;"
+ "}"
```

Tamaño: **840.882 bytes**.

Los 65.537 bindings requieren al menos:

```text
65537 * 4 = 262148 > 262144
```

Ticks de IR, limpieza y otros registros solo adelantan el rechazo.

Para este patrón escalar y lineal, la inferencia de cada binding procesa como máximo su restricción y el par escalar correspondiente; el flujo visita construcción, inicialización y limpieza; efectos visita esas operaciones locales, sin aristas de llamada ni rondas no triviales. Aun usando una cota conservadora de ocho cargos por binding más 64 de infraestructura, cada presupuesto anterior queda bajo:

```text
8 * 65537 + 64 = 524360 < 1000000
```

Los planes terminales compartidos del punto 2 son necesarios para esa cota; no se permite introducir joins artificiales de todos los prefijos de fallo. Funciones, anidamiento y tipos permanecen pequeños.

#### Evidencia exigida

Estas familias son obligaciones normativas, no evidencia de una ejecución nueva. Cada implementación debe conservar:

- Generador y tamaño real de sus bytes.
- En negativos de cuota, NX-E0003, `budget_kind`, `budget_scope`, `incomplete_analysis`, límite y perfil normal; en positivos, aceptación sin diagnósticos.
- Primer cargo excedido, `observed = limit + 1`; propiedad informa contadores local/global y continuación posterior dentro del saldo global.
- `primary_span` no nulo, dentro del archivo y no vacío, comprobado mediante la propiedad de §13.13, sin exigir un offset igual al del modelo independiente.
- Contadores de las etapas anteriores, mostrando que no agotaron otra cuota.

Una implementación que cambie estas unidades lógicas o el algoritmo de rondas debe revisar el perfil y las expectativas. No puede mantener el identificador y sustituir estas recetas por fallos de otra guarda.
## 12. Tres flujos reducidos

### 12.1. Herramienta: EX01 reducido

```nexo
fn sumar(records: Array<Result<i64, i64>>) -> Result<i64, i64>
    with no_alloc, no_block, no_suspend, no_io {
    var i = 0
    var total = 0
    while i < len(records) {
        let value = records[i]?
        total = try_add(total, value)?
        i = i + 1
    }
    return Ok(total)
}

fn main() -> Result<(), i64> {
    let file = mock_open(7)
    let records = mock_read_records(file)?
    let total = sumar(records)?
    mock_write_i64(total)?
    return Ok(())
}
```

El entorno simulado suministra, como datos de entrada del arnés:

- Un resultado de lectura.
- Una secuencia arbitraria de resultados de registro.
- El resultado de escritura.

No selecciona una respuesta por nombre de archivo fixture ni por texto fuente. El bucle Nexo debe recorrer y sumar la secuencia recibida.

La reducción omite CLI de aplicación, texto, líneas, trim e implementación de parsing. Un registro válido se representa por `Ok(i64)`; uno mal formado por `Err(20)`. Las líneas vacías se omiten al preparar la entrada del modelo. Esto no acredita texto ni parsing de EX01 completo.

Casos obligatorios:

| Entrada | Resultado |
|---|---|
| `Ok([Ok(12), Ok(8), Ok(5)])`, escritura exitosa | Escribe `25`, retorna `Ok(())`, cierra el recurso una vez. |
| Lectura de secuencia vacía | Escribe `0`. |
| `Err(10)` de lectura | Retorna `Err(10)`, no escribe, limpia. |
| `[Ok(12), Err(20), Ok(5)]` | Retorna `Err(20)`, no procesa el registro posterior ni escribe. |
| `[Ok(9223372036854775807), Ok(1)]` | Retorna `Err(30)`, sin `R0101`. |
| Escritura fallida | Registra el intento y retorna `Err(40)`; no registra salida exitosa. |

Se repite con longitudes y valores generados, no solo con estos ejemplos.

### 12.2. Actualización de datos

```nexo
fn incrementar(inout xs: Array<i64>)
    with no_alloc, no_block, no_suspend, no_io {
    var i = 0
    while i < len(xs) {
        xs[i] = xs[i] + 1
        i = i + 1
    }
}

fn main() {
    var original = [1, 2]
    var copia = original
    incrementar(inout copia)
    assert_eq(original, [1, 2])
    assert_eq(copia, [2, 3])
}
```

La copia ocurre antes del contrato de `incrementar`. El análisis comprueba también la limpieza y las lecturas indexadas del cuerpo. No demuestra CoW ni unicidad física general.

### 12.3. Petición simulada

```nexo
fn responder(route: i64) -> i64
    with no_alloc, no_block, no_suspend, no_io {
    return if route == 0 {
        value: 200
    } else {
        value: 404
    }
}

fn main() -> Result<(), i64> {
    let route = mock_request()?
    mock_reply(responder(route))?
    return Ok(())
}
```

Se prueban ambas rutas y errores de recepción/respuesta. No hay HTTP, callbacks retenidos, scheduler ni cancelación.


### 12.4. Interfaz determinista de entrada y observación

Esta interfaz es parte del diseño del corte y debe implementarse antes de acreditar los tres flujos.

#### Invocación y selección de perfil

```text
nexo check archivo.nx [--profile ID] [--format text|json]
nexo run archivo.nx [--mock-env entorno.json] [--profile ID] [--format text|json]
```

Los valores predeterminados son `--profile f0-0002-v2` y `--format text`. Solo se aceptan los perfiles publicados en §11. El entorno mock se admite únicamente en `run`.

El orden es: validar opciones, leer y comprobar la fuente, comprobar la entrada de `run`, leer y validar el entorno mock, ejecutar. Un error previo impide las etapas siguientes. No se selecciona una respuesta por ruta del programa, nombre de fixture ni texto fuente.

#### Esquema del entorno

El archivo es JSON UTF-8, de hasta 1.048.576 bytes, con este objeto exacto:

```json
{
  "schema_version": 1,
  "mock_read_records": [
    {"ok": [{"ok": "12"}, {"ok": "8"}, {"ok": "5"}]}
  ],
  "mock_write_i64": [
    {"ok": null}
  ],
  "mock_request": [],
  "mock_reply": []
}
```

Los cinco campos son obligatorios; no se admiten campos adicionales ni claves duplicadas. El orden de las claves no importa. Si se omite `--mock-env`, se utiliza el mismo esquema con las cuatro colas vacías.

Los tipos JSON se definen así:

```text
I64         = cadena decimal canónica de un i64
Registro    = {"ok": I64} | {"err": I64}
Lectura     = {"ok": [Registro, ...]} | {"err": I64}
Unidad      = {"ok": null} | {"err": I64}
Peticion    = {"ok": I64} | {"err": I64}

mock_read_records = [Lectura, ...]
mock_write_i64    = [Unidad, ...]
mock_request      = [Peticion, ...]
mock_reply        = [Unidad, ...]
```

Cada objeto de resultado contiene exactamente una de sus claves. `I64` usa `0` o un entero con signo opcional `-`, sin `+`, ceros iniciales ni `-0`, y debe estar dentro del rango `i64`. No se admiten números JSON para representar valores Nexo `i64`.

Cada primitiva consume la siguiente respuesta de su propia cola global, en orden de llamadas. `mock_read_records` usa la cola independientemente de la etiqueta del recurso recibido. Una cola agotada produce `Err(-9000)` y registra el evento de error correspondiente; no inventa un diagnóstico R ni detiene el controlador.

El parser JSON debe ser acotado y rechazar una profundidad mayor de 16 antes de recursión no acotada. La estructura válida anterior queda muy por debajo de ese límite. JSON inválido, esquema inválido, archivo ilegible o excesivo son errores de configuración del controlador, con salida 2, sin ejecutar el programa. Sus razones son respectivamente `invalid_mock_json`, `invalid_mock_schema`, `mock_env_unreadable` y `mock_env_too_large`. No se les asigna un código NX-E de programa.

#### Eventos exactos

Los eventos son cadenas ASCII sin prefijo de transporte ni salto final. Los enteros se escriben en decimal canónico; las identidades de recurso son enteros no negativos, asignados desde cero por adquisición comprometida.

| Operación | Éxito | Error tipado |
|---|---|---|
| `mock_read_records(r)` | `read_records:<id>:ok:<cantidad>` | `read_records:<id>:err:<error>` |
| `mock_write_i64(n)` | `write_i64:<n>:ok` | `write_i64:<n>:err:<error>` |
| `mock_request()` | `request:ok:<valor>` | `request:err:<error>` |
| `mock_reply(status)` | `reply:<status>:ok` | `reply:<status>:err:<error>` |

Cada llamada que se compromete produce **un evento**, tanto si devuelve `Ok` como `Err`. Ese evento registra el intento y su resultado. Una escritura exitosa añade después una línea de salida con el entero; una escritura fallida no añade salida. `mock_reply` solo registra el evento; no genera una línea adicional de salida.

Se mantienen los eventos históricos `open:<id>:<label>`, `inspect:<id>:<label>` y `close:<id>:<label>`. `mock_blocking_open` utiliza las mismas formas; su contrato de destrucción conserva `block, io`, sin introducir una espera real ni un segundo evento de cierre.

Antes de comprometer una primitiva se comprueban las cuotas necesarias para su respuesta y sus observaciones. Si falla una cuota, no se consume la respuesta de la cola, no se asigna identidad, no se emite un evento parcial y no se registra una escritura exitosa. Los efectos ya producidos al evaluar argumentos no se revierten.

#### Salida de datos

`print` y la salida exitosa de `mock_write_i64` producen payloads de línea sin salto almacenado:

- `i64`: decimal canónico.
- `bool`: `true` o `false`.
- Unidad: `()`.
- Array: `[` y `]`, elementos separados por `, `, con formato recursivo.
- `Result` de datos: `Ok(<payload>)` o `Err(<payload>)`, con formato recursivo.

Los recursos y los tipos que pueden contenerlos no pertenecen al dominio de `print`.

#### Bytes observables y reservas

Existe un único registro ordenado `observations`. Cada entrada es:

```text
{"channel":"stdout","text":PAYLOAD}
{"channel":"event","text":EVENTO}
```

La codificación virtual de cada entrada es exacta:

| Entrada | Bytes contabilizados |
|---|---|
| Salida | UTF-8 de `PAYLOAD + "\n"`. |
| Evento `open`, `inspect` o `close` | UTF-8 de `"mock-resource:" + EVENTO + "\n"`. |
| Evento de las cuatro primitivas nuevas | UTF-8 de `"mock-io:" + EVENTO + "\n"`. |

El salto es un único byte LF, también en Windows. Se suman salida **y** eventos. No se cuentan los bytes del envoltorio JSON, su escape de caracteres, diagnósticos, ayuda ni la línea final de resultado del controlador.

La ocupación de la cuota es:

```text
observation_bytes_emitidos + reserved_observation_bytes
```

Antes de abrir un recurso se comprueba adquisición más cierre futuro. La reserva contiene el tamaño exacto de la línea `close` para esa identidad y etiqueta. Al cerrar, esos bytes pasan de reservados a emitidos. El cierre no necesita ampliar la cuota y no puede omitirse por agotamiento de trabajo.

Por ejemplo, `open:0:7` ocupa 23 bytes con prefijo y LF; su cierre ocupa 24. La adquisición compromete 47 bytes aunque inicialmente solo 23 estén emitidos.

#### Formato JSON de ejecución

Con `--format json`, la CLI escribe en stdout exactamente un objeto JSON seguido de LF; no intercala allí las líneas del programa. Los resultados controlados no escriben simultáneamente una segunda representación en stderr.

Los campos obligatorios del objeto son:

```text
schema_version: 1
profile_id: ID
outcome: resultado definido abajo
stdout: lista de payloads de salida, sin LF
events: lista de eventos, sin prefijo ni LF
observations: registro ordenado de ambas clases
observation_bytes: bytes efectivamente emitidos según la codificación virtual
reserved_observation_bytes: reservas pendientes; cero al terminar la limpieza
diagnostics: lista de diagnósticos estructurados del catálogo
```

`stdout` y `events` son las proyecciones de `observations`, no fuentes independientes de verdad. El orden de claves JSON no es significativo; orden de listas y cadenas sí lo es.

Formas de `outcome`:

```text
{"kind":"unit"}                         main devuelve ()
{"kind":"ok"}                           main devuelve Ok(())
{"kind":"err","error":I64}               main devuelve Err(i64)
{"kind":"simulator_fault"}              fallo R; diagnostics contiene la causa
{"kind":"rejected"}                     rechazo estático; diagnostics contiene la causa
{"kind":"input_error"}                  fuente ilegible o UTF-8 inválido
{"kind":"driver_error","reason":RAZON}   opciones o entorno mock inválidos
{"kind":"checked"}                      check satisfactorio
```

`input_error` conserva `NX-E0004` y su subcausa. Entre las razones de `driver_error` se incluyen las cuatro del entorno, `invalid_arguments` y `unknown_profile`. Una IR inválida sigue siendo fallo interno de implementación, no un resultado Nexo ni un R de programa.

Ejemplo literal para:

```nexo
fn main()->Result<(),i64>{
    mock_write_i64(25)?;
    return Ok(());
}
```

con una respuesta `{"ok":null}` en `mock_write_i64`:

```json
{"schema_version":1,"profile_id":"f0-0002-v2","outcome":{"kind":"ok"},"stdout":["25"],"events":["write_i64:25:ok"],"observations":[{"channel":"event","text":"write_i64:25:ok"},{"channel":"stdout","text":"25"}],"observation_bytes":27,"reserved_observation_bytes":0,"diagnostics":[]}
```

El evento ocupa 24 bytes y la salida tres. Una respuesta `{"err":"40"}` produce solo `write_i64:25:err:40`, `outcome = {"kind":"err","error":"40"}` y ninguna línea de salida.

#### Formato de texto y códigos de salida

En formato de texto, los payloads salen por stdout con LF; los eventos salen por stderr con los prefijos anteriores y LF. El registro ordenado del modelo es normativo para el orden entre canales; no se infiere ese orden del buffering de dos descriptores del anfitrión.

Al terminar normalmente se escribe en stderr una de estas líneas de presentación, excluida de la cuota:

```text
nexo-result:()
nexo-result:Ok(())
nexo-result:Err(40)
```

El error decimal de la última forma es el devuelto por `main`.

| Situación | Código de salida |
|---|---:|
| `check` satisfactorio; `main` devuelve `()` u `Ok(())` | 0 |
| `main` devuelve `Err`; fallo R; rechazo estático, incluido exceso de fuente | 1 |
| Uso/configuración inválidos; fuente ilegible o UTF-8 inválido; fallo interno controlado de implementación | 2 |

Un `Err` final no lleva diagnóstico R ni NX-E. Los formatos de texto y JSON describen la misma ejecución, con las mismas cuotas y limpieza.

### 12.5. Legibilidad de EX01–EX04

Esta tabla distingue reconocimiento de los archivos de aceptación y ejecución del perfil. Los ejemplos completos no se sustituyen ni se declaran aceptados.

Los offsets indicados se calculan sobre el contenido de cada bloque Nexo de la base, sin delimitadores Markdown y normalizado a LF para esta fixture documental. Si se prueba otro archivo con CRLF, se recalculan sobre sus bytes reales.

| Ejemplo completo | Reconocimiento y primer rechazo en F0-0002 | Correspondencia reducida y límite |
|---|---|---|
| EX01, `total.nx` | Primer `use`, `[0,3)`: `NX-E0006`, `feature_id = modules_visibility`. `value` en `let value` conserva categoría Name, `[355,360)`. `app.Error` es una ruta de tipo reconocida, no un binding desconocido. | §12.1 sustituye archivo/texto/parsing por respuestas tipadas; ejecuta el recorrido y suma. No acredita EX01 completo. |
| EX02, `salud.nx` | Primer `use`, `[0,3)`: mismo rechazo de módulos. Atributos, `struct`, `String`, vistas, métodos y texto siguen diferidos. | §12.3 prueba decisión y respuesta con enteros. No implementa HTTP, JSON ni servidor. |
| EX03, `particulas.nx` | Primer `use`, `[0,3)`: mismo rechazo de módulos. Traits, tipos de usuario, `f32`, layouts, proyecciones y preparación de vistas no se aceptan. | §12.2 prueba copia independiente y modificación exclusiva de arrays. No acredita layouts ni EX03 completo. |
| EX04, `cuadrados.nx` | Primer `use`, `[0,3)`: mismo rechazo de módulos. `value` en `for value` conserva categoría Name, `[244,249)`. `gen`, defaults, `u32`, `u64`, rango y `yield` siguen diferidos. | Generadores y EX04 reducido requieren F0-0003. No hay sustitución por array materializado en este corte. |

El primer rechazo impide atribuir cobertura semántica al resto del archivo. Las exclusiones de tipos se comprueban por separado con F29–F31 de §14; quitar los imports de un ejemplo no basta para convertirlo en positivo.

## 13. Diagnósticos y migración

Se usa el [catálogo único integrado](F0-CODIGOS.md), con trazabilidad por sección de este ADR. Los estáticos emitidos usan los códigos canónicos `NX-E...`.

Campos mínimos adicionales:

- `profile_id` y `stage`.
- `explanation_id` y explicación consultable mediante `nexo explain`.
- Tipos esperados/encontrados cuando corresponda.
- Identidades de declaraciones y permisos en cadenas causales.
- Metadatos de límites y efectos.
- `feature_id` para `NX-E0006`, siempre separado de `subcause`.
- `runtime_budget_kind` para `R0002`; `budget_kind` únicamente para presupuestos de análisis.

Los diagnósticos de alcance no se mezclan con reglas semánticas todavía no implementadas.

Las sugerencias no añaden `unsafe`, no eliminan contratos y no insertan copias o transferencias sin explicar sus consecuencias. Solo llevan estado aplicable después de recomprobar el archivo editado. Insertar `;` no es reparación completa si deja una función sin retorno.

### 13.1. NB8: `value:` vacío

Para un `ValueStmt` reconocido sin operando:

1. `invalid_value_target` si no tiene destinatario directo válido.
2. En caso contrario, `missing_value_operand`.
3. Después, `non_final_branch_value`.
4. Después, errores de flujo derivados.

En `{ value: }`, `missing_value_operand` prevalece sobre el terminador ausente derivado de esa misma sentencia vacía. Se señala `value:` y se relaciona `}`.

En `{ value: 1 }` el operando sí existe: corresponde `NX-E0005` en `}`.

Esta excepción no permite aceptar el bloque ni suprimir errores sintácticos independientes. La prioridad entre tipos de ramas se fija en §4.7.

### 13.2. Conciliación de los 13 códigos E históricos

No se reescriben ADR, corpus, resultados ni evidencias de F0-0001. La tabla describe el destino en el perfil nuevo, no una modificación de aquellas emisiones.

| Código de F0-0001 | Destino en F0-0002 |
|---|---|
| `E0001` | `NX-E0001` para un token realmente desconocido. Una construcción reconocida con feature_id publicado usa NX-E0006. Los operadores reconocidos `*`, `!=`, `>`, `<=` y `>=`, sin producción aceptada ni feature_id, usan NX-E0002 en posición binaria; no conservan artificialmente un fallo léxico. |
| `E0002` | `NX-E0002` para sintaxis inválida. La limitación «única `fn main`» se retira: varias funciones y bibliotecas son válidas. `run` usa `NX-E0112` para ausencia o firma inválida de entrada. Un terminador ausente reconocido usa `NX-E0005`; una construcción reconocida diferida usa `NX-E0006`. |
| `E0003` | `NX-E0003`, con el presupuesto y perfil publicados. Las dos guardas históricas de longitud de cadena siguen registradas sin promesa de emisión independiente. |
| `E0004` | `NX-E0004/source_unreadable` o `NX-E0004/invalid_utf8`. Se conserva salida 2 y prioridad de tamaño antes de decodificación. |
| `E0101` | `NX-E0101`; se amplía expresamente a destino de llamada no declarado. |
| `E0102` | `NX-E0102/type_mismatch`, `not_indexable` o `resource_required`, según causa. La antigua restricción a transferir/destruir solo recursos se retira; véase §13.3. |
| `E0103` | `NX-E0103` cuando no puede inferirse el elemento del array vacío. Una anotación ahora puede resolverlo. |
| `E0104` | `NX-E0104`; conserva duplicación de bindings y se amplía a funciones y parámetros duplicados. |
| `E0105` | `NX-E0105` por modificación sin permiso mutable. |
| `E0107` | `NX-E0107` por literal no representable como `i64`. |
| `E0201` | `NX-E0201` por copia lógica no admisible. |
| `E0202` | `NX-E0202/definitely_consumed`; los caminos nuevos pueden exigir `maybe_consumed`. Una fuente que todavía contiene `move` falla antes por alcance. |
| `E0203` | Retirado de las emisiones de `f0-0002-v2`; se conserva su significado histórico en el catálogo. Destinos de sus cuatro subcausas en §13.3. |

No existe `E0106` en el inventario histórico; no se crea una correspondencia por completar la numeración.

### 13.3. Las cuatro limitaciones de NX-E0203

| Subcausa histórica | Decisión del perfil nuevo |
|---|---|
| `unsupported_move_position` | La grafía `move` se rechaza en cualquier posición con `NX-E0006`, `feature_id = legacy_move`. Después de migrar a `consume`, transferir un binding completo en un contexto propietario ya no está limitado al inicializador. Una proyección consumida usa `feature_id = partial_transfer`; un receptor prefijado de llamada, `receiver_mode`. |
| `unsupported_resource_position` | Recursos temporales, argumentos, retornos y payloads de `Result` pasan a estar admitidos. Un array que contiene recursos transitivamente usa `NX-E0006`, `feature_id = resource_array`. |
| `unsupported_resource_rebinding` | Retirada: la reasignación completa de un recurso en `var` está permitida, con limpieza del valor reemplazado. Una escritura sobre `let` conserva `NX-E0105`. |
| `unsupported_transfer_kind` | Retirada: `consume` y `drop` se admiten sobre datos propietarios. La fuente con `move` conserva el rechazo de migración `legacy_move`; no se atribuye al tipo del dato. |

NX-E0203 no se reutiliza para otra regla ni se elimina del registro histórico.

### 13.4. Migración archivo por archivo del corpus

Las rutas son las del corpus de F0-0001. Las fuentes originales se preservan únicamente como evidencia histórica. Las fixtures activas se sustituyen conjuntamente por su versión oficial al migrar; sus comentarios se actualizan con ellas. Los originales solo se conservan como evidencia histórica, sin otro camino de producción.

| Archivo | Resultado histórico | Fuente sin editar bajo F0-0002 | Fixture migrada y expectativa |
|---|---|---|---|
| `corpus/pass/integer_limits.nx` | Aceptado | Aceptado | Sin cambio; mismas aserciones y salida `25`. |
| `corpus/pass/nested_copy.nx` | Aceptado | Aceptado | Sin cambio; mismas observaciones. La lectura indexada deja de copiar toda la base, por lo que cambia la contabilidad interna publicada. |
| `corpus/pass/resource_transfer.nx` | Aceptado | `NX-E0006`, `feature_id = legacy_move`, sobre `move` | Sustituir `move source` por `consume source`; aceptado, mismos eventos de adquisición, inspección y cierre. |
| `corpus/pass/return_cleanup.nx` | Aceptado | Aceptado | Sin cambio; limpia al retornar y no imprime `999`. |
| `corpus/pass/shadow_scope.nx` | Aceptado | `NX-E0006`, `feature_id = legacy_move`, sobre `move` | Sustituir `move r` por `consume r`; aceptado, mismo sombreado y eventos. |
| `corpus/fail/escape_scope.nx` | `E0101` | `NX-E0101` | Sin cambio de fuente; actualizar el esperado de la copia migrada. |
| `corpus/fail/immutable_write.nx` | `E0105` | `NX-E0105` | Sin cambio de fuente. |
| `corpus/fail/mixed_array.nx` | `E0102` | `NX-E0102/type_mismatch` | Sin cambio de fuente; señalar `true`. |
| `corpus/fail/nested_resource.nx` | `E0203` | `NX-E0006`, `feature_id = resource_array` | Misma fuente; span del array completo `[mock_open(1)]`. No esperar NX-E0203. |
| `corpus/fail/overflow_literal.nx` | `E0107` | `NX-E0107` | Sin cambio de fuente. |
| `corpus/fail/resource_copy.nx` | `E0201` | `NX-E0201` | Sin cambio de fuente; copia de `r`. |
| `corpus/fail/trailing_garbage.nx` | `E0002` | `NX-E0002` | Sin cambio de fuente; token `ignored` tras un módulo válido, esperado una declaración o EOF. No es rechazo por haber terminado una única `main`. |
| `corpus/fail/unreachable_resource_copy.nx` | `E0201` | `NX-E0201` | Sin cambio de fuente; el código posterior a `return` sigue comprobándose. |
| `corpus/fail/use_after_drop.nx` | `E0202` | `NX-E0202/definitely_consumed` | Sin cambio de fuente; señalar el uso de `r` en `inspect`. |
| `corpus/fail/use_after_move.nx` | `E0202` | `NX-E0006`, `feature_id = legacy_move`, sobre `move` | Sustituir únicamente `move r` por `consume r`; después se exige `NX-E0202/definitely_consumed` en el `r` de `inspect`. |

Un rechazo por `move` no cuenta como regresión aprobada de uso posterior al consumo. La copia migrada debe alcanzar ese uso.

### 13.5. Conciliación de los cinco códigos R y pruebas de presupuesto

Los códigos conservan sus causas bajo `diagnostic_namespace = F0-0002-sim`; no se cambian las expectativas históricas bajo `F0-0001`.

| Código histórico | Destino y condición |
|---|---|
| `R0001` | Trabajo agotado, con la contabilidad nueva identificada por perfil. La regresión migrada usa `fn main(){let r=mock_open(7);while true{}}`; exige `open:0:7`, `close:0:7`. |
| `R0101` | Desbordamiento de suma `i64`; la misma operación `+` sigue fallando. `try_add` es otra operación y devuelve `Err(30)`. |
| `R0102` | Índice fuera de límites, después de evaluar el RHS en una asignación. |
| `R0201` | Aserción de igualdad fallida sobre datos de tipos compatibles. |
| `R0301` | Ejecución de `fail()`, seguida de limpieza. |

La fuente generada por `tests/corpus.rs::work_budget_exhaustion_closes_resources`, con `a0=[1]` y duplicación hasta `a24`, se conserva como antecedente. Su versión migrada espera `R0002`, `runtime_budget_kind = live_value_cells`, y cierre único del recurso 7. Se añade la regresión de bucle de la tabla para conservar cobertura real de `R0001`.

`R0002` y `R0103` son adiciones del corte, no renombrados de los cinco códigos históricos. Un `Err` devuelto por `main` usa la interfaz y salida de §12.4, sin código R.

Los arneses antiguos que esperaban rechazo de `if`, efectos, varias funciones, reasignación de recursos o `drop` de datos requieren expectativas migradas explícitas. No se obtiene un PASS actual cambiando silenciosamente un esperado histórico.


### 13.6. Esquema literal de un diagnóstico JSON

Esta es la serialización normativa de cada elemento de `diagnostics` del informe de §12.4. Se aplica tanto a NX-E como a R. No se serializan las estructuras privadas de Rust directamente.

`String` significa cadena Unicode válida. `UInt` significa número JSON entero no negativo, representable exactamente hasta `9007199254740991`; no se emiten fracciones ni exponentes para estos campos. Los valores Nexo `i64` siguen siendo cadenas cuando aparecen como datos del programa.

Todos los campos de `Diagnostic` que se muestran a continuación son obligatorios. La ausencia de valor se expresa mediante `null` o lista vacía, según el tipo. No se omiten silenciosamente. `data` admite únicamente las claves registradas más abajo. Otros campos requieren revisar la versión del esquema.

```text
Span = {
  "file": String,
  "start": UInt,
  "end": UInt
}

RelatedSpan = {
  "role": String,
  "span": Span
}

Entity = {
  "kind": "function" | "parameter" | "binding" | "temporary"
          | "loan" | "intrinsic" | "type",
  "id": String
}

Cause = {
  "kind": "call" | "copy" | "consume" | "borrow" | "reborrow"
          | "drop" | "return" | "backedge" | "effect" | "limit",
  "operation": String,
  "span": Span | null,
  "entity": Entity | null,
  "evidence": "verified" | "external_assumption" | "unknown"
}

Token = {
  "kind": "name" | "keyword" | "symbol" | "integer" | "decimal"
          | "string" | "eof" | "invalid",
  "text": String | null,
  "span": Span
}

Edit = {
  "span": Span,
  "expected_text": String,
  "replacement": String
}

RepairValidation = {
  "profile_id": String,
  "mode": "check" | "run_entry",
  "result": "accepted" | "rejected"
}

Repair = {
  "id": String,
  "status": "proposed" | "applicable",
  "description": String,
  "edits": [Edit, ...],
  "validation": RepairValidation | null,
  "consequences": [String, ...]
}

Diagnostic = {
  "schema_version": 1,
  "diagnostic_namespace": "Nexo-proposed-v1" | "F0-0002-sim",
  "profile_id": String,
  "code": String,
  "subcause": String | null,
  "rule": String,
  "explanation_id": String | null,
  "severity": "error",
  "stage": "input" | "lex" | "parse" | "names" | "arguments"
           | "types" | "ownership" | "effects" | "lower"
           | "entry" | "runtime",
  "message": String,
  "operation": String,
  "primary_span": Span | null,
  "related_spans": [RelatedSpan, ...],
  "cause_chain": [Cause, ...],
  "feature_id": String | null,
  "unexpected_token": Token | null,
  "expected_tokens": [String, ...],
  "expected_tokens_kind": "admissible_set" | "canonical_insertion" | "lexical_category" | null,
  "parser_context": String | null,
  "recognized_prefix": Span | null,
  "repair": [Repair, ...],
  "data": Data
}
```

#### Identidad, campos y orden

- NX-E usa `diagnostic_namespace = Nexo-proposed-v1`; R usa `F0-0002-sim`.
- `code` y `subcause` pertenecen al catálogo. R lleva `subcause = null`.
- `rule` es el identificador `code` o `code/subcause`. Para NX-E, `explanation_id` coincide con `rule`; para R es `null` en este corte.
- `feature_id` es no nulo exclusivamente para NX-E0006 y pertenece a sus 30 identificadores emitibles.
- `message` es presentación en una sola línea, sin controles de terminal. No identifica la regla ni sustituye campos estructurados.
- `operation`, `parser_context`, `role` e IDs de reparación son identificadores estables, no mensajes traducidos.
- El orden de claves de cualquier objeto JSON es **no significativo**. No se admiten claves duplicadas. El arnés compara objetos decodificados.
- `related_spans` se ordena por archivo, inicio, fin y `role`; se eliminan duplicados idénticos.
- `cause_chain` conserva orden causal: desde la operación denunciada hacia su origen. No se ordena por offsets. Una cadena directa sin pasos adicionales es `[]`.
- `expected_tokens` usa lexemas literales o categorías como `Name` e `IntLiteral`. En `admissible_set` no hay duplicados y se ordena por bytes UTF-8; `canonical_insertion` conserva la secuencia de inserción, por ejemplo `[";"]`.
- `repair` se ordena por `id`. Sus ediciones se refieren simultáneamente a la fuente original, son no solapadas y se presentan por archivo/inicio/fin. Para aplicarlas se procede de derecha a izquierda dentro de cada archivo.

Los doce valores de `RelatedSpan.role`, incluido `preceding_expression`, y sus reglas de emisión se enumeran exclusivamente en §13.15.

Las identidades fuente son reproducibles: `function:<offset>`, `parameter:<offset>` y `binding:<offset>` usan el comienzo del nombre declarado. Préstamos y temporales usan `loan:<offset>:<ordinal>` y `temporary:<offset>:<ordinal>`, con ordinal desde cero en el orden del lowering canónico de ese sitio. Intrínsecos usan `prelude:<nombre>`. Los tipos usan su escritura canónica sin espacios. Estas identidades son locales al archivo indicado por sus spans, no direcciones de memoria.

#### Convención de spans

`start` y `end` son offsets **desde cero en bytes UTF-8 originales**, con intervalo semiabierto `[start,end)`. No son índices UTF-16, columnas ni números de caracteres. Se exige `start <= end <= bytes_leídos`.

`file` es la cadena de ruta recibida por la CLI, sin convertirla silenciosamente a absoluta ni cambiar mayúsculas o separadores. La API recibe explícitamente ese identificador; para una fuente sin ruta utiliza `<memory>`.

En texto UTF-8 válido, los extremos de spans emitidos están en fronteras de caracteres. CRLF conserva dos bytes. EOF se representa mediante `start = end = longitud_en_bytes`. Una inserción también usa un intervalo vacío.

Excepciones de entrada:

- `source_unreadable`: `primary_span = null`, con `data.input_path`.
- `invalid_utf8`: span del rango de bytes inválidos; no se exige que ese rango decodifique.
- `source_bytes`: señala exactamente el primer byte excedido, `[limit,limit+1)`, en la lectura acotada anterior a la decodificación; no se exige que ese byte aislado decodifique.

No se corrigen offsets inválidos de un diagnóstico ya construido para presentarlos como evidencia válida. El constructor/serializador debe rechazarlos como defecto interno. La robustez del formateador frente a datos internos inválidos se prueba por separado.

`unexpected_token` es obligatorio como objeto para NX-E0001/NX-E0002/NX-E0005 cuando hay bytes o token identificables. EOF usa `kind = eof`, `text = ""` y span vacío. Bytes no decodificables usan `kind = invalid`, `text = null` y su representación hexadecimal en `data`.

`parser_context` y `recognized_prefix` son obligatoriamente no nulos para NX-E0002/NX-E0005. El prefijo señala los bytes de la producción ya reconocidos hasta el comienzo del token inesperado. Las exclusiones de alcance no se convierten por ello en errores sintácticos: pueden dejar esos campos en `null`.

#### Campos permitidos de `data`

Las claves no pertinentes se omiten dentro de `data`; el objeto puede ser `{}`.

| Claves | Tipo y obligación |
|---|---|
| `input_path` | String; obligatorio para fuente ilegible. |
| `invalid_bytes_hex` | String de pares hexadecimales minúsculos; obligatorio al representar bytes inválidos sin texto. |
| `budget_kind` | String de la enumeración cerrada del catálogo; obligatorio para NX-E0003 y prohibido para R0002. |
| `budget_scope` | `function` o `compilation`; obligatorio para NX-E0003. Solo `ownership_flow_steps` admite `function`; las guardas de tamaño/profundidad usan `compilation`. |
| `incomplete_analysis` | Uno de `input`, `parse`, `types`, `ownership`, `effects`, `lower`, igual a `stage`; obligatorio para NX-E0003. |
| `analysis_function` | Entity de función; obligatoria solo en NX-E0003 con `ownership_flow_steps` y `budget_scope=function`, ausente en los demás diagnósticos. |
| `runtime_budget_kind` | `call_frames`, `live_value_cells` u `observation_bytes`; obligatorio para R0002. |
| `limit`, `observed` | UInt; obligatorios para NX-E0003, R0001 y R0002. |
| `work_consumed` | UInt; obligatorio para R0001 y equivalente al trabajo concedido, no al lote solicitado. |
| `declaration_kind` | `binding`, `function` o `parameter`; obligatorio para NX-E0104. |
| `use_kind` | `value` o `call_target`; obligatorio para NX-E0101. |
| `expected_type`, `found_type` | String según §13.17; obligatorios solo para NX-E0102 (`type_mismatch`, `not_indexable`, `resource_required`) y NX-E1303; ausentes en los demás códigos, incluido NX-E1317. |
| `unresolved_components` | Lista de String; obligatoria para NX-E0113. |
| `type_spelling` | String; obligatoria para `feature_id = unsupported_type`. |
| `pending_scope` | `migration`, `F0-0003` o `later_adr`; obligatorio para NX-E0006. |
| `entry` | String; `"main"` para NX-E0112. |
| `effect_model` | String; `f0-0002-sim-v1` en este perfil. |
| `effect_evidence` | `known_effect` o `unknown_effect`. |
| `contract_origin` | Span de la cota incumplida. |
| `required_effect` | `alloc`, `block`, `suspend` o `io`. |
| `origin`, `loan`, `binding`, `parameter` | Entity; obligatorias o ausentes exclusivamente según la tabla de NX-E1105 siguiente. |
| `active_permission` | `read` o `inout`, exclusivamente para NX-E1105 y según su subcausa. |
| `requested_access` | `read`, `write`, `consume`, `drop` o `borrow_inout`, exclusivamente para NX-E1105 y según su subcausa. |

Los cuatro campos de efectos son obligatorios exclusivamente para NX-E0412–NX-E0415. Las claves de identidad y permiso de las tres filas anteriores están ausentes en los demás códigos. `callee` no se emite dentro de `data` en este perfil: el destino de una llamada se identifica mediante Entity en su paso causal.

| Subcausa de NX-E1105 | Claves obligatorias de `data` | Claves de identidad ausentes | `active_permission` → `requested_access` permitidos |
|---|---|---|---|
| `permission_escalation` | `origin`, `parameter`, `active_permission`, `requested_access` | `binding`, `loan`, `callee` | `read` → `write`, `consume`, `drop`, `borrow_inout`; `inout` → `consume`, `drop`. |
| `active_borrow_conflict` | `origin`, `binding`, `loan`, `active_permission`, `requested_access` | `parameter`, `callee` | `read` → `write`, `consume`, `drop`, `borrow_inout`; `inout` → `read`, `write`, `consume`, `drop`. |
| `overlapping_mutable_borrows` | `origin`, `binding`, `loan`, `active_permission`, `requested_access` | `parameter`, `callee` | Únicamente `inout` → `borrow_inout`. |

Estas son las claves completas de `data` para NX-E1105; no se añaden otras. `read` significa acceder por lectura o solicitar un préstamo de lectura; `write`, asignar; `consume`, transferir; `drop`, destruir; `borrow_inout`, solicitar un préstamo exclusivo mediante un argumento `inout`. La solicitud rechazada no crea un LoanId.

En elevación, `parameter` y `origin` identifican el mismo parámetro del cuerpo comprobado; `active_permission` es el permiso declarado de ese parámetro, aunque no exista un préstamo local vivo. En conflicto, `binding` y `origin` identifican la raíz declarada en el cuerpo comprobado —Entity de binding o parámetro— y `loan` identifica el permiso vivo incompatible elegido por §13.15; `active_permission` es el de ese préstamo, no el permiso propietario de la raíz. El intento se clasifica por su operación explícita, antes de una lectura auxiliar del Place. Las reglas de disponibilidad e inmutabilidad conservan sus prioridades.

Para los préstamos de argumentos de este perfil, el offset de `loan:<offset>:0` es el inicio del argumento completo que creó el permiso, incluida su etiqueta o prefijo si existen; cada argumento crea como máximo un permiso directo. El ID es estático y no cambia entre invocaciones. Los représtamos conservan sus vínculos causales; no convierten un préstamo nuevo compatible en un conflicto.

Las fixtures P04–P07 fijan los objetos completos siguientes, sin claves adicionales:

| Fixture | `data` exacta |
|---|---|
| P04 | `{"origin":{"kind":"parameter","id":"parameter:5"},"parameter":{"kind":"parameter","id":"parameter:5"},"active_permission":"read","requested_access":"consume"}` |
| P05 | `{"origin":{"kind":"binding","id":"binding:46"},"binding":{"kind":"binding","id":"binding:46"},"loan":{"kind":"loan","id":"loan:52:0"},"active_permission":"inout","requested_access":"borrow_inout"}` |
| P06 | `{"origin":{"kind":"binding","id":"binding:40"},"binding":{"kind":"binding","id":"binding:40"},"loan":{"kind":"loan","id":"loan:46:0"},"active_permission":"read","requested_access":"borrow_inout"}` |
| P07 | `{"origin":{"kind":"binding","id":"binding:46"},"binding":{"kind":"binding","id":"binding:46"},"loan":{"kind":"loan","id":"loan:56:0"},"active_permission":"inout","requested_access":"borrow_inout"}` |

`pending_scope = migration` corresponde exclusivamente a `legacy_move`; `F0-0003`, a `closure`, `generator`, `option` y `match`; los otros 25 feature_id usan `later_adr`. Para `match`, esa asignación identifica el corte que diseñará la selección necesaria para consumir `next` y `Option` de §1; no promete allí toda la selección por patrones de la base.

#### Reparaciones

`status = applicable` exige `validation.result = accepted` con el mismo perfil y comprobación completa. `mode = run_entry` añade la comprobación de entrada, sin ejecutar el programa ni sus mocks.

`validation = null` significa que la edición no se ha recomprobado. Una validación rechazada no puede anunciarse aplicable. `expected_text` debe coincidir exactamente con los bytes decodificados del span original; una inserción exige cadena vacía.

Los campos `description` y `consequences` explican cambios de valor, copia, transferencia, efectos o interfaz pertinentes. No sustituyen la validación.

### 13.7. Ejemplos completos de diagnósticos

Los siguientes objetos son elementos completos de `diagnostics`, no informes exteriores abreviados. Los mensajes son textos de ejemplo; la comparación de identidad utiliza los campos estructurados.

#### NX-E0005 con token y reparación propuesta

Fuente exacta de `terminador.nx`, sin LF final:

```nexo
fn main(){let x=1}
```

```json
{
  "schema_version": 1,
  "diagnostic_namespace": "Nexo-proposed-v1",
  "profile_id": "f0-0002-v2",
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
  "unexpected_token": {
    "kind":"symbol",
    "text":"}",
    "span":{"file":"terminador.nx","start":17,"end":18}
  },
  "expected_tokens": [";"],
  "expected_tokens_kind": "canonical_insertion",
  "parser_context": "statement_terminator",
  "recognized_prefix": {"file":"terminador.nx","start":10,"end":17},
  "repair": [
    {
      "id":"insert_terminator",
      "status":"proposed",
      "description":"Insertar un terminador explícito.",
      "edits":[
        {
          "span":{"file":"terminador.nx","start":17,"end":17},
          "expected_text":"",
          "replacement":";"
        }
      ],
      "validation":null,
      "consequences":["Termina el binding antes del cierre del bloque."]
    }
  ],
  "data": {}
}
```

#### NX-E0006 con `feature_id`

Fuente exacta de `decimal.nx`:

```nexo
fn main(){let x=0.5;}
```

```json
{
  "schema_version": 1,
  "diagnostic_namespace": "Nexo-proposed-v1",
  "profile_id": "f0-0002-v2",
  "code": "NX-E0006",
  "subcause": null,
  "rule": "NX-E0006",
  "explanation_id": "NX-E0006",
  "severity": "error",
  "stage": "parse",
  "message": "Literal flotante fuera del perfil.",
  "operation": "syntax.parse",
  "primary_span": {"file":"decimal.nx","start":16,"end":19},
  "related_spans": [],
  "cause_chain": [],
  "feature_id": "float_literal",
  "unexpected_token": null,
  "expected_tokens": [],
  "expected_tokens_kind": null,
  "parser_context": null,
  "recognized_prefix": null,
  "repair": [],
  "data": {"pending_scope":"later_adr"}
}
```

#### R0101

Fuente exacta de `suma.nx`:

```nexo
fn main(){print(9223372036854775807+1);}
```

```json
{
  "schema_version": 1,
  "diagnostic_namespace": "F0-0002-sim",
  "profile_id": "f0-0002-v2",
  "code": "R0101",
  "subcause": null,
  "rule": "R0101",
  "explanation_id": null,
  "severity": "error",
  "stage": "runtime",
  "message": "Desbordamiento de suma i64.",
  "operation": "value.add",
  "primary_span": {"file":"suma.nx","start":16,"end":37},
  "related_spans": [],
  "cause_chain": [],
  "feature_id": null,
  "unexpected_token": null,
  "expected_tokens": [],
  "expected_tokens_kind": null,
  "parser_context": null,
  "recognized_prefix": null,
  "repair": [],
  "data": {}
}
```

El informe exterior usa respectivamente `outcome.kind = rejected`, `rejected` y `simulator_fault`, con salida 1. En estos tres ejemplos las listas de salida, eventos y observaciones están vacías.

### 13.8. Presentación textual exacta

El modo texto conserva una primera línea compatible con búsquedas por código y posición, usando ahora el código canónico:

```text
CODE:LINE:COLUMN: MESSAGE
```

Línea y columna son desde uno. La línea cuenta LF; CRLF constituye un salto. La columna cuenta valores escalares Unicode desde el último LF, no bytes, unidades UTF-16 ni anchura visual; un tabulador cuenta como un carácter. Cuando no hay texto válido con el que calcular posición se usan `0:0`.

Después se escriben las líneas siguientes, en este orden. `J(x)` significa JSON compacto con claves de objetos ordenadas lexicográficamente, sin espacios exteriores ni escapes innecesarios de caracteres Unicode. Cada línea acaba en LF, también en Windows.

```text
  namespace: DIAGNOSTIC_NAMESPACE
  profile: PROFILE_ID
  stage: STAGE
  operation: OPERATION
  at: J(PRIMARY_SPAN)
```

Luego:

1. `  subcause: SUBCAUSE`, solo si no es nulo.
2. `  feature_id: FEATURE_ID`, solo si no es nulo.
3. Una línea `  related: J(RELATED_SPAN)` por elemento, en el orden del array.
4. Una línea `  cause: J(CAUSE)` por elemento.
5. Si hay `unexpected_token`, una línea `  syntax: J(SYNTAX)`, donde `SYNTAX` contiene exactamente `unexpected_token`, `expected_tokens`, `expected_tokens_kind`, `parser_context` y `recognized_prefix`.
6. `  data: J(DATA)`, solo si el objeto no está vacío.
7. Una línea `  repair: J(REPAIR)` por reparación.

No hay colores, secuencias ANSI ni líneas de contexto fuente añadidas implícitamente en este formato contractual. `rule` y `explanation_id` se recuperan de código/subcausa según §13.6; el formato JSON conserva además sus campos explícitos.

Los diagnósticos van a stderr, después de las observaciones y de la limpieza. No se escribe una línea `nexo-result` para un rechazo o fallo R. `Err` normal conserva la línea de resultado de §12.4.

`check` satisfactorio escribe en stdout exactamente:

```text
nexo-check:ok
```

seguido de LF, con stderr vacío. Los errores de configuración sin diagnóstico de programa escriben `nexo-driver:<reason>` y LF en stderr. No fabrican un NX-E.

Así, el test Unicode de §13.11 debe comenzar exactamente por `NX-E0101:3:10:`. La traducción de `message` puede variar sin cambiar código, posiciones ni campos estructurados.

### 13.9. Cardinalidad y orden de diagnósticos

`diagnostics` es una lista, pero no acumula cascadas de etapas que ya no pudieron comprobarse correctamente.

Orden de etapas:

```text
input
→ lex
→ parse, incluidos terminadores y formas reconocedoras
→ names
→ arguments
→ types
→ ownership
→ effects
→ lower/verificación
→ entry, solo para run
→ lectura del entorno mock
→ runtime
```

Una etapa que produce errores impide las posteriores y la ejecución. El orden es de barreras semánticas, no obliga a abortar otras funciones independientes de la misma etapa. La prioridad causal se aplica antes de emitir consecuencias, aunque las comprobaciones internas estén distribuidas entre módulos.

- Entrada, lexer y parser se detienen en su primer fallo, según el recorrido publicado.
- En una etapa semántica se conservan errores de sitios independientes alcanzados sin utilizar una declaración o expresión inválida. Se evita analizar descendientes o consecuencias que solo dependen del error ya emitido.
- Los resultados de una misma etapa se ordenan por `file`, `primary_span.start`, `primary_span.end`, `code` y `subcause`, tratando `null` como cadena vacía para el desempate. No se ordenan por el texto traducido.
- Un presupuesto local agotado interrumpe el análisis de esa función; se conservan solo diagnósticos de estados convergidos y se continúa con las posteriores dentro del presupuesto global (§11.1). Un presupuesto global agotado detiene el resto. Cada uso tiene una entrada; la lista sin NX-E0003 es un subconjunto exacto de la ejecución sin cuota, también en subcausa y testigos. No se afirma haber examinado lo restante ni se entrega un artefacto verificado.
- En runtime se conserva únicamente el primer fallo R. La limpieza obligatoria no añade diagnósticos R secundarios.

En cada llamada, la asociación de argumentos tiene una regla adicional precisa:

1. Recorre los argumentos escritos de izquierda a derecha.
2. En el primer defecto de etiqueta, posición, duplicación o modo, emite **un diagnóstico para esa llamada** y detiene su asociación.
3. Solo si todos los argumentos presentes son estructuralmente válidos se buscan parámetros ausentes, por orden de declaración; se informa el primero.
4. No se comparan tipos ni efectos de una llamada cuya asociación falló.

Por tanto, S03 produce una lista de longitud uno con `unknown_argument_label`; no añade `missing_argument`. S05 produce una lista de longitud uno con `positional_after_named`; no añade la ausencia derivada de `y`. Dos llamadas independientes con esos defectos sí pueden producir dos diagnósticos de la etapa `arguments`.

La comprobación hipotética de §6.2 no emite NX-E0202 si su única causa es exigir disponibilidad a un valor inexistente por `Never`. Se conserva NX-E0202 cuando existe un consumo independiente que alcanza el uso en el estado hipotético; la supresión no se aplica a todo código inalcanzable ni oculta contradicciones de tipos.

En `ownership` se comprueba disponibilidad antes de copia y permiso antes de cualquier consecuencia de escape. NX-E0201 y NX-E1105 no se adelantan de forma que oculten el NX-E0202 correspondiente al mismo acceso.

NX-E0105 frente a NX-E0202 sigue prioridad **causal**, nunca numérica. Si la indisponibilidad impide el acceso requerido y hace derivado el diagnóstico de modificación, se emite NX-E0202 con su subcausa y testigo. Una escritura completa sobre `let` infringe mutabilidad incluso después de consumo: sustituir el valor no exige leer el anterior disponible, por lo que corresponde NX-E0105; la reinicialización completa sobre `var` sí se permite. Dos operaciones independientes que infringen disponibilidad y mutabilidad conservan ambos diagnósticos, ordenados por spans. §14.11 fija los casos; no se escoge siempre el menor código ni se borran errores de una sentencia por otro error independiente.

### 13.10. Formato de `nexo explain`

```text
nexo explain NX-Edddd [--subcause nombre] [--format text|json]
```

El formato predeterminado es `text`. Sin `--subcause`, se devuelve una entrada por subcausa registrada, o una entrada con `subcause = null` cuando la regla no tiene subcausas, salvo NX-E0006. Con la opción, solo se devuelve la entrada seleccionada. Las entradas se ordenan por subcausa, con `null` primero.

Para NX-E0006, tanto en JSON como en texto, se devuelven exactamente **30 entradas**, una por `feature_id`, ordenadas por ese ID en bytes UTF-8. Cada entrada tiene `subcause = null`, `explanation_id = NX-E0006`, `feature_ids = [ID]`, `availability = in_profile` y `example_profile_id = f0-0002-v2`: lo incluido es el diagnóstico de exclusión, no la característica rechazada. `description` y `repair_consequences` son específicas de ese ID. `negative_source` es la fuente sin marcadores de la primera fila por número de F01–F33 con ese ID; `positive_source` es su positivo completo tras aplicar la edición publicada. Así, `unsupported_type` usa F29 y `methods` usa F23. Un `--subcause` para NX-E0006 devuelve `unknown_subcause`; un feature_id no se convierte en subcausa.

Formato JSON: un objeto UTF-8 y LF en stdout, stderr vacío y salida 0:

```text
{
  "schema_version": 1,
  "diagnostic_namespace": "Nexo-proposed-v1",
  "code": String,
  "entries": [
    {
      "subcause": String | null,
      "explanation_id": String,
      "availability": "in_profile" | "outside_profile" | "historical",
      "description": String,
      "negative_source": String,
      "positive_source": String,
      "example_profile_id": String | null,
      "feature_ids": [String, ...],
      "repair_consequences": [String, ...]
    }
  ]
}
```

Todos esos campos son obligatorios. `availability` describe el alcance del diseño, no acredita implementación. `example_profile_id = null` identifica ejemplos documentales fuera de los perfiles ejecutables publicados; no se anuncian como positivos aceptados por F0-0002. `feature_ids` es la lista ordenada de IDs pertinentes, vacía salvo cuando corresponda.

En texto, cada entrada ocupa este bloque, con valores JSON compactos donde se indica `J`, una línea vacía entre entradas y un LF final:

```text
EXPLANATION_ID
availability: AVAILABILITY
example_profile_id: J(EXAMPLE_PROFILE_ID)
rule: DESCRIPTION
negative_source: J(NEGATIVE_SOURCE)
positive_source: J(POSITIVE_SOURCE)
feature_ids: J(FEATURE_IDS)
repair_consequences: J(REPAIR_CONSEQUENCES)
```

`description` es una sola línea. Las fuentes completas conservan sus saltos dentro de las cadenas JSON, sin convertir ejemplos incompletos en pseudocódigo.

Código inexistente, subcausa inexistente y argumentos inválidos terminan con salida 2. En texto escriben en stderr `nexo-explain:unknown_code`, `nexo-explain:unknown_subcause` o `nexo-explain:invalid_arguments`, más LF. En JSON escriben un único objeto:

```text
{"schema_version":1,"error":{"reason":String,"code":String|null,"subcause":String|null}}
```

No hay consulta de red, ejecución de ejemplos ni diagnósticos NX-E inventados para un error de uso de `explain`.

### 13.11. Spans pendientes del corpus y migración de tests

#### Cuatro spans de migración

Para fijar bytes reproducibles, estas copias migradas conservan el cuerpo exacto del archivo histórico y cambian únicamente su primera línea a `// expect: NX-Edddd`. La tabla incluye el comentario. No lleva BOM.

| Archivo | Token y ubicación inequívoca | Código | Span con LF | Span con CRLF |
|---|---|---|---|---|
| `escape_scope.nx` | `r` de `inspect(⟦r⟧)` | NX-E0101 | `[66,67)` | `[67,68)` |
| `immutable_write.nx` | raíz de `⟦value⟧[0][0] = 2` | NX-E0105 | `[51,56)` | `[52,57)` |
| `overflow_literal.nx` | `⟦9223372036854775808⟧` | NX-E0107 | `[44,63)` | `[45,64)` |
| `unreachable_resource_copy.nx` | inicializador de `let duplicate = ⟦r⟧` | NX-E0201 | `[78,79)` | `[79,80)` |

Si un arnés conserva el comentario histórico sin `NX-`, debe restar tres bytes a esos offsets. No puede cambiar el comentario y reutilizar los offsets anteriores. El salto final del archivo no modifica estos cuatro rangos.

#### Tests de integración

Los tests históricos se conservan únicamente como evidencia documental del primer corte; la suite activa migra sus fuentes y expectativas en un único corte. El helper de ejecución se adapta al artefacto de IR y a los canales de resultado publicados, sin interpretar todo `Err` como fallo R.

| Test o grupo actual | Migración exigida |
|---|---|
| `tests/corpus.rs::positive_corpus` | Leer el corpus migrado; los pasivos con `move` deben usar `consume`. Comprobar resultado normal y ausencia de diagnósticos, además de poder compilar. |
| `negative_corpus_exact_codes` | Comparar código, subcausa, etapa, span y datos pertinentes. No basta un comentario que contenga solo el código antiguo. Aplicar §§13.4 y 13.11. |
| `nested_copy_output` | Mantener `[[1, 9], [7, 8]]`; adaptar únicamente la lectura del canal de salida de la API. |
| `transfer_has_one_owner_and_exact_cleanup` | Usar `resource_transfer.nx` migrado. Conservar exactamente los seis eventos históricos. |
| `early_return_unwinds_scopes_in_reverse_order` | Conservar los cuatro eventos y ausencia de salida; comprobar el resultado unidad del canal nuevo. |
| `recoverable_failure_and_partial_initialization` | Conservar R0101 y los cuatro eventos; añadir namespace, etapa y span de la suma. |
| `explicit_close_and_failure_do_not_double_close` | Conservar R0301 y el orden de eventos. No interpretarlo como un `Result::Err` del programa. |
| `indexing_and_overflow_are_checked_in_every_build` | Conservar R0101/R0102 y cierre; comparar los diagnósticos estructurados en ambos builds. |
| `assignment_bounds_preserve_cleanup` y `assignment_checks_negative_bounds_after_rhs` | Conservar prioridad de evaluación y eventos. Añadir spans de la operación que realmente falla. |
| `statements_after_return_still_checked` | Cambiar esperado E0101 a NX-E0101; exigir el span de `unknown`, aunque esté después de `return`. |
| `malformed_inputs_return_diagnostics` | Quitar `""` de los negativos de compilación: es biblioteca vacía válida. Añadir `check` satisfactorio y `run` con NX-E0112/missing_main en EOF `[0,0)`. El caso `1.5` pasa a NX-E0006 con `feature_id = float_literal`; las truncaciones restantes siguen probando su causa sintáctica, no cualquier error. |
| `diagnostic_unicode_comment_location` | Para la fuente exacta `// á🙂\nfn main(){\n let a = missing;\n}`, exigir NX-E0101, span `[30,37)`, línea 3, columna 10 y prefijo textual `NX-E0101:3:10:`. |
| `nesting_limit_is_diagnostic` | E0003 pasa a NX-E0003, con `budget_kind = syntax_nesting`, límite, perfil y span. |
| `composed_ast_depth_is_bounded_before_recursive_passes` | NX-E0003 con `budget_kind = expression_tree_depth`; no atribuirlo a una guarda de longitud de cadena. |
| `inferred_nesting_across_bindings_is_bounded` | NX-E0003 con `budget_kind = array_type_depth`; aplicar su prioridad frente a profundidad de valor simultánea. |
| `diagnostic_render_handles_arbitrary_utf8_offsets` | Separar validación de spans y presentación. En `🙂é`, las fronteras válidas son 0, 4 y 6. Los demás offsets del test no pueden serializarse como spans válidos; su rechazo controlado no debe provocar pánico. Para spans válidos se usa prefijo NX-E0001. |
| `work_budget_exhaustion_closes_resources` | Separar crecimiento por copias, ahora R0002/live_value_cells, y bucle sin crecimiento, R0001, como §13.5. Añadir las fronteras numéricas de §11.7. |
| `truncated_and_mutated_inputs_never_panic` | Cambiar la semilla válida de `move r` a `consume r`; conservar la semilla legada como prueba separada de alcance. Una mutación semántica no se acredita si solo falla por `move`. |
| `tests/cli.rs::cli_separates_check_from_run_and_preserves_error_codes` | Usar los pasivos migrados. `check` debe mantener stderr vacío. Para `use_after_move.nx` migrado a `consume`, exigir `NX-E0202/definitely_consumed`; el prefijo de texto es `NX-E0202:`. La fuente legada sin editar exige NX-E0006/feature_id legacy_move en un test separado. |
| `cli_invalid_invocations_do_not_execute` | Conservar salida 2. Archivo ausente: prefijo `NX-E0004:` y subcausa source_unreadable en JSON. Invocaciones inválidas usan el formato de controlador, sin código NX-E. |
| `cli_size_limit_precedes_utf8_decoding_of_truncated_input` | Conservar salida 1 y prioridad de tamaño. Cambiar prefijo a `NX-E0003:` y comprobar `budget_kind = source_bytes`; no aceptar invalid_utf8. |

La futura suite que use `--format json` debe decodificar el objeto exterior y sus diagnósticos. Las comprobaciones textuales restantes verifican el formato de §13.8; no buscan `E0202:` o `E0101:` como subcadenas que podrían coincidir accidentalmente dentro de otro dato.


### 13.12. Tabla única de etapas y operaciones diagnósticas

Esta tabla completa normativamente §§13.4 y 14. Evita repetir columnas en cada fixture. Todas las filas de esas secciones heredan la etapa y operación aquí asignadas; no dependen del módulo Rust que detecte el problema.

Una fila agrupada aplica a todas sus subcausas emitibles, salvo las excepciones escritas. Los códigos y subcausas del catálogo no enumerados aquí no se emiten en este perfil.

| Código o selección | `stage` | `operation` |
|---|---|---|
| NX-E0001 | `lex` | `syntax.lex` |
| NX-E0002; NX-E0005 | `parse` | `syntax.parse` |
| NX-E0003 | Según `budget_kind`, tabla siguiente | `analysis.limit` |
| NX-E0004/source_unreadable | `input` | `input.read` |
| NX-E0004/invalid_utf8 | `input` | `input.decode` |
| NX-E0006 | Según característica y contexto, tabla siguiente | Según esa tabla |
| NX-E0101 | `names` | `names.resolve` |
| NX-E0104, incluidos los tres `declaration_kind` | `names` | `names.declare` |
| NX-E0102/type_mismatch, not_indexable, resource_required | `types` | `types.check` |
| NX-E0103; NX-E0113 | `types` | `types.infer` |
| NX-E0105 | `ownership` | `place.write` |
| NX-E0107 | `types` | `value.literal` |
| NX-E0111, sus siete subcausas | `arguments` | `call.bind` |
| NX-E0112, sus dos subcausas | `entry` | `entry.check` |
| NX-E0201 | `ownership` | `value.copy` |
| NX-E0202, sus dos subcausas | `ownership` | `ownership.use` |
| NX-E1105, sus tres subcausas | `ownership` | `ownership.access` |
| NX-E1302/missing_else, invalid_value_target, missing_value_operand, non_final_branch_value | `parse` | `syntax.parse` |
| NX-E1302/missing_branch_value | `ownership` | `flow.branch` |
| NX-E1303 | `types` | `types.check` |
| NX-E1317 | `ownership` | `flow.return` |
| NX-E0417/duplicate_operation, invalid_operation, duplicate_restriction, empty_operation_list, unrestricted_mixed | `parse` | `syntax.parse` |
| NX-E0412–NX-E0415 | `effects` | Operación denunciada, según §13.15 |
| R0001; R0002 | `runtime` | `runtime.limit` |
| R0101 | `runtime` | `value.add` |
| R0103 | `runtime` | `value.sub` |
| R0102 | `runtime` | `value.index` |
| R0201 | `runtime` | `intrinsic.assert_eq` |
| R0301 | `runtime` | `intrinsic.fail` |

R0102 señala el selector `[indice]` concreto que falla, incluidos corchetes; en accesos anidados no abarca selectores que tuvieron éxito ni el prefijo `inout`. Es la misma regla para lectura, escritura y préstamo. §6.1 conserva índices y RHS antes de comprobar límites de escritura; §13.15 relaciona acceso, destino o argumento completo. R0001/R0002 por unidad final implícita señala la `}` del cuerpo. R0002/`call_frames` señala la llamada rechazada completa (§11.3).

`non_callable_target` no necesita inferir el tipo completo de un binding: en este perfil ningún binding puede contener un valor invocable aceptado. La identidad resuelta permite rechazarlo en `arguments`.

`missing_branch_value` requiere determinar continuaciones normales. Tipos conserva esa obligación de control pendiente, sin inventar un tipo para el valor ausente ni emitir NX-E0113 por él. La obligación se resuelve o diagnostica en `ownership`, antes de producir un artefacto aceptado. Esto no retrasa los defectos puramente sintácticos de `value:`.

Las subcausas no emitibles de NX-E0417 siguen siendo las de §14.4. NX-E0203, NX-E0102/field_requires_grouped_call y las reglas de closures, tareas, módulos o traits diferidos conservan su registro, pero no adquieren una etapa emitible por esta tabla.

| `budget_kind` emitible de NX-E0003 | `stage` |
|---|---|
| `source_bytes` | `input` |
| `syntax_nesting` | `parse` |
| `expression_tree_depth` | `parse` |
| `function_count` | `parse` |
| `array_type_depth` | `types` |
| `value_type_depth` | `types` |
| `type_solver_steps` | `types` |
| `ownership_flow_steps` | `ownership` |
| `effect_fixpoint_steps` | `effects` |
| `ir_items` | `lower` |

Estos diez valores agotan las emisiones de presupuestos de análisis del perfil. Los perfiles reducidos heredan sus etapas. La recomprobación posterior al lowering conserva la etapa propietaria del presupuesto; no lo renombra a `lower`. Esa recomprobación es una revisión del artefacto dentro de la barrera de verificación de §13.9, no una nueva ejecución de etapas fuente ni un reinicio de contadores.

| Selección de NX-E0006 | `stage` / `operation` |
|---|---|
| `function_value` | `names` / `names.resolve` |
| `inferred_non_unit_result`, `resource_array`, `option`, `unsupported_type` | `types` / `types.check` |
| `async` cuando identifica un tipo escrito de futuro/tarea | `types` / `types.check` |
| `partial_transfer` por argumento proyectado del intrínseco `drop` | `arguments` / `call.bind` |
| `legacy_move`, `float_literal`, `range`, `tuple`, `attributes`, `default_argument`, `closure`, `receiver_mode`, `no_heap_alloc`, `match`, `user_types`, `traits_generics`, `modules_visibility`, `extensions`, `provenance_clauses`, `comptime`, `text`, `methods`, `compound_assignment`, `for_iteration`, `loop_control`, `generator`, `region` | `parse` / `syntax.parse` |
| `partial_transfer` por prefijo `consume`; `async` en los restantes contextos reconocidos | `parse` / `syntax.parse` |

Así, F01–F32 conservan sus etapas publicadas y F33 usa `parse` / `syntax.parse`. La ampliación de `drop` no cambia la etapa de F11.

### 13.13. Ubicaciones de presupuestos y excepción de comparación

Los seis presupuestos estructurales de la tabla conservan un `primary_span` exacto. Para `type_solver_steps`, `effect_fixpoint_steps`, `ir_items` y `ownership_flow_steps`, el arnés independiente compara exactamente `code`, `stage`, `data.budget_kind`, `data.limit`, `data.observed`, `data.budget_scope`, `data.incomplete_analysis` y, cuando corresponda, `data.analysis_function`, pero **no compara por igualdad `primary_span`**. Para cada emisión comprueba como propiedad que el span no sea nulo, pertenezca al archivo de entrada y cumpla `0 <= start < end <= longitud_utf8`, con extremos en fronteras de caracteres. El compilador elige una ubicación fuente asociada al trabajo rechazado; no se prescribe su offset. Esta libertad no exime de las unidades de §11.1 y la contabilidad de §11.8. Esta excepción rige en perfiles normales y reducidos y en las comparaciones con debug y release. No altera etapas, cuotas, contadores, recorridos ni alcanzabilidad.

| `budget_kind` | Regla de `primary_span` |
|---|---|
| `source_bytes` | `[limit,limit+1)`. Se leen como máximo `limit+1` bytes para decidir este exceso; `observed = limit+1`, aunque el archivo real sea mayor. Tiene prioridad sobre decodificación. |
| `syntax_nesting` | Token de entrada de la activación lógica que elevaría la profundidad a `limit+1`; EOF usa su span vacío. |
| `expression_tree_depth` | Span completo del nodo de expresión candidato cuya incorporación excedería la altura, antes de enlazarlo. |
| `function_count` | Nombre de la primera función que excede el número permitido, al reconocer su declaración. |
| `array_type_depth` | Span de la construcción fuente a la que se está asignando el tipo excesivo: tipo escrito completo si se comprueba una anotación; expresión completa si se sintetiza su tipo. |
| `value_type_depth` | La misma regla de origen que `array_type_depth`, conservando la prioridad de este último si ambos fallan. |
| `type_solver_steps`, `effect_fixpoint_steps`, `ir_items`, `ownership_flow_steps` | Propiedad de span válido, fuente y no vacío definida arriba; sin offset esperado ni comparación de igualdad contra el modelo independiente. |

Las comprobaciones de profundidad usan el valor candidato; no fabrican un span sobre un tipo internado sin fuente.

#### Activaciones lógicas de sintaxis

`syntax_nesting` cuenta únicamente entradas activas a `Block`, `Expr` y `Type` de §3. No cuenta funciones auxiliares de precedencia ni llamadas accidentales del parser Rust. Una entrada permanece activa mientras se analizan sus hijos:

- Un cuerpo de función empieza con un `Block`.
- Un inicializador, RHS de asignación, argumento, índice, condición, operando de retorno o de `value:` entra en `Expr`.
- Elementos de array y expresiones agrupadas entran en sus `Expr` hijos.
- Los argumentos de un tipo constructor entran en sus `Type` hijos.
- Un `IfForm` en posición de sentencia no añade un `Expr` exterior; sus condiciones sí.
- Cada entrada se cobra antes de consumir su primer token. Las producciones envolventes de precedencia no añaden entradas.

En la fuente exacta de `nesting_limit_is_diagnostic`, `"fn main(){let x=" + "["*200 + "1" + "]"*200 + ";}"`, el cuerpo ocupa una activación y el inicializador otra. El intento de activación 129 ocurre en el corchete número 128, span **`[143,144)`**. Esperado: NX-E0003, `stage = parse`, `budget_kind = syntax_nesting`, `limit = 128`, `observed = 129`.

La regresión equivalente sobre un RHS usa `"fn main(){var x=0;x=" + "["*200 + "1" + "]"*200 + ";}"`: prefijo de 20 bytes, activación 129 en el corchete 128, span `[147,148)`, con el mismo código, etapa, presupuesto, límite y observed. Añadir esta activación no cambia el caso del inicializador: `16 + 127 = 143`. El positivo sustituye el RHS completo por `0`: `fn main(){var x=0;x=0;}`.

La altura de expresión cuenta un átomo como 1 y cada nodo compuesto como `1 + max(alturas de hijos)`; la agrupación no añade altura. Para una llamada se incluye el destino y sus argumentos; para `if` expresión, condición y expresiones de sus ramas. La guarda usa los spans originales del candidato, incluidos los paréntesis fuente que lo delimitan.

Parser y lowering usan esa misma altura, sin añadir llamadas auxiliares, bloques contenedores ni paréntesis. Caso normativo ASCII sin LF final, con literales sujetos a F0-D15:

```text
T = "(if true{value:1;}else{value:1;})"
fuente(n) = "fn main(){let x=" + unir n copias de T mediante "+" + ";}"
```

Cada T tiene altura 2. `fuente(127)` tiene 4.335 bytes y altura 128: aceptación, `diagnostics=[]`, y ejecución unidad sin observaciones. `fuente(128)` tiene 4.369 bytes y altura 129: una NX-E0003 en `parse`, `operation=analysis.limit`, `data={"budget_kind":"expression_tree_depth","budget_scope":"compilation","incomplete_analysis":"parse","limit":128,"observed":129}`, primario `[16,4367)`. Solo excede altura de expresión, no anidamiento sintáctico. El positivo no puede devolver `OutsideScalarSlice("lowering depth")`. Con términos atómicos `1`, 127 y 128 términos se aceptan; 129 exceden altura 128. La guarda interna `depth >= 128` registrada en I07a debe alinearse con esta medida en I10a.

Ninguna de estas guardas se diagnostica como NX-E0001/NX-E0002; sus campos sintácticos permanecen nulos o vacíos.

### 13.14. Contrato completo de los negativos léxicos y sintácticos

#### Léxico

Para NX-E0001:

```text
stage = lex
operation = syntax.lex
parser_context = lexical_token
expected_tokens = ["valid_token"]
expected_tokens_kind = lexical_category
recognized_prefix = [0, primary_span.start)
```

`valid_token` significa una unidad tokenizable por el lexer de este perfil, no una enumeración de terminales aceptados por el parser.

Un carácter no reconocido ocupa todos sus bytes UTF-8 y usa `unexpected_token.kind = invalid`, con `text` igual a ese carácter. Un lexema comenzado pero incompleto ocupa desde su comienzo hasta el punto donde falla, incluido EOF cuando corresponda; `text` contiene esos bytes decodificados. Los bytes no UTF-8 se rechazan antes con NX-E0004/invalid_utf8.

La copia migrada de K01, `fn main() { £ }`, exige NX-E0001, token `£`, span `[12,14)`, prefijo `[0,12)` y los campos anteriores. El positivo es `fn main() { }`. Su namespace actual es `Nexo-proposed-v1`; la evidencia histórica permanece bajo F0-0001.

#### Listas auxiliares

Las siguientes listas están ordenadas por bytes UTF-8:

```text
ExprStart = [
  "(", "-", "IntLiteral", "Name", "[",
  "consume", "false", "if", "inout", "true"
]

BlockNext = [
  "(", "-", "IntLiteral", "Name", "[",
  "consume", "false", "if", "inout", "let",
  "return", "true", "var", "while", "{", "}"
]
```

Representan entradas de la gramática aceptada. Los reconocedores de características diferidas se ejecutan antes de informar un error genérico y conservan sus códigos específicos. No se incluyen en estas listas como si fueran construcciones aceptadas.

#### Filas exactas

Cada rango de esta tabla se convierte en un objeto Span con el identificador de archivo de la fixture. Para NX-E0002 se usa siempre `stage = parse`, `operation = syntax.parse`, `expected_tokens_kind = admissible_set`. `unexpected_token.span` coincide con `primary_span`.

| Fuente o fila | Token y span primario | `parser_context` | `expected_tokens` | `recognized_prefix` |
|---|---|---|---|---|
| `fn` | EOF, `[2,2)` | `function_name` | `["Name"]` | `[0,2)` |
| `fn main(` | EOF, `[8,8)` | `parameter_list` | `[")","Name","consume","inout"]` | `[7,8)` |
| `fn main(){` | EOF, `[10,10)` | `block_item` | `BlockNext` | `[9,10)` |
| `fn main(){let a=` | EOF, `[16,16)` | `binding_initializer` | `ExprStart` | `[10,16)` |
| `fn main(){let a=-` | EOF, `[17,17)` | `signed_integer` | `["IntLiteral"]` | `[16,17)` |
| `fn main(){let a=[1,` | EOF, `[19,19)` | `array_element` | `ExprStart` | `[16,19)` |
| `fn main(){let a=1;} garbage` | Name `garbage`, `[20,27)` | `module_item` | `["EOF","fn"]` | `[0,20)` |
| `trailing_garbage.nx` migrado, comentario NX-E0002 y LF | Name `ignored`, `[33,40)` | `module_item` | `["EOF","fn"]` | `[0,33)` |
| El mismo archivo con CRLF | Name `ignored`, `[34,41)` | `module_item` | `["EOF","fn"]` | `[0,34)` |
| S22 | Keyword `match`, `[14,19)` | `binding_name` | `["Name"]` | `[10,14)` |
| S27 | Symbol `*`, `[17,18)` | `unsupported_binary_operator` | `["+","-","<","=="]` | `[10,17)` |
| S28 | Symbol `!=`, `[17,19)` | `unsupported_binary_operator` | `["+","-","<","=="]` | `[10,17)` |
| S30 | Integer `1`, `[38,39)` | `inout_place` | `["Place"]` | `[32,38)` |
| S31 | Integer `1`, `[24,25)` | `consume_place` | `["Place"]` | `[16,24)` |
| S32/S33/S34 | Operador marcado en §14.9 | `unsupported_binary_operator` | `["+","-","<","=="]` | `[10,17)` |
| Copia migrada de K02: `fn main() ;` | Symbol `;`, `[10,11)` | `function_suffix` | `["->","with","{"]` | `[0,10)` |

EOF tiene `kind = eof` y `text = ""`. El positivo de K02 es `fn main() {}`. La lista histórica de F0-0001, que solo esperaba `{`, no se reutiliza como lista exhaustiva del perfil nuevo.

El caso vacío de `malformed_inputs_return_diagnostics` sigue siendo biblioteca válida. Su caso decimal sigue siendo NX-E0006, no NX-E0001/NX-E0002.

#### Regla para los restantes truncamientos y mutaciones

No se calcula `expected_tokens` a partir de un mensaje del parser. Se deriva de la gramática de §3 y de la única `WithClause` de D11:

1. Se aplican primero los reconocedores específicos de alcance, de prefijos inválidos, de operadores excluidos y de NX-E0005.
2. Para el error genérico se conserva la derivación parcial de la producción comprometida. Una asignación se selecciona únicamente cuando un Place va seguido de `=`; en otro caso se conserva la expresión. La selección contextual de `value:` y de `if` sigue §3.
3. Los tokens esperados son `FIRST` del resto de esa producción, incluyendo continuaciones de producciones anulables y EOF cuando sea admisible. Se expanden auxiliares de EBNF; Name e IntLiteral permanecen categorías. La lista no tiene duplicados y se ordena por bytes.
4. Se usa la producción comprometida más interna. Entre alternativas que todavía comparten el mismo prefijo se toma la unión de continuaciones; no se escoge una alternativa por el token erróneo. Un empate de producción usa el orden de declaración de la EBNF.
5. Fuera de los contextos específicos de la tabla anterior, `parser_context` es `grammar.P`, donde `P` es exactamente un no terminal de §3 o uno de `WithClause`, `OperationList`, `OperationLimit`, `Operation`, `Restrictions`, `RestrictionItem`, `Restriction`. Es un conjunto finito, no el nombre de una función interna del parser.
6. `recognized_prefix` comienza en el primer token de esa producción y termina en el comienzo del token inesperado, incluyendo trivia intermedia. Si aún no se consumió ningún token de la producción, es un rango vacío en ese punto. Para Program comienza en byte cero.

Las filas exactas prevalecen como nombres estables de sus contextos. El mecanismo general cubre las mutaciones restantes sin depender del stack de implementación.

Para NX-E0005, `parser_context = statement_terminator`, `expected_tokens = [";"]` y `expected_tokens_kind = canonical_insertion`. El prefijo empieza en la sentencia y acaba en el siguiente token o EOF. La expresión precedente debe aparecer en `related_spans` según §13.15. B07/B08 y el ejemplo de §13.7 heredan esta misma regla.

### 13.15. Vocabularios cerrados, ubicaciones y cadenas

Los campos declarados como String en §13.6 quedan restringidos por las enumeraciones siguientes para este perfil. Los textos de presentación continúan siendo traducibles.

#### `Diagnostic.operation`

El vocabulario cerrado es la unión de los literales de la tabla de §13.12 y los siguientes cinco valores de efectos:

| Valor adicional | Emisión en NX-E0412–NX-E0415 |
|---|---|
| `function.call` | El sitio denunciado llama a una función Nexo. |
| `intrinsic.call` | El sitio denunciado llama a un intrínseco, salvo el tratamiento específico de `drop`. |
| `value.build_array` | La construcción local del array aporta el efecto. |
| `value.copy` | La copia local aporta el efecto. |
| `value.drop` | Destrucción explícita o limpieza implícita que aporta el efecto. |

No se serializan nombres de funciones de usuario como valores de `operation`; se representan mediante Entity. Un wrapper no cambia el código de efectos: cambia el sitio denunciado y añade pasos causales.

#### `RelatedSpan.role`

Enumeración cerrada:

```text
declaration
previous_declaration
contract
origin
consumption
backedge
exit
expected_type
found_type
context
delimiter
preceding_expression
```

Se emiten únicamente las ubicaciones que exige esta tabla; el resto de reglas tiene `related_spans = []`. Después se aplica la ordenación y eliminación de duplicados de §13.6.

| Regla | Ubicaciones relacionadas obligatorias |
|---|---|
| NX-E0005 | `preceding_expression`: expresión completa cuyo terminador falta. En Binding es el inicializador; en asignación, RHS; en return/value:, su operando; en ExprStmt, la expresión. Si la sentencia válida no tiene expresión, se usa `context` sobre su introductor, sin inventar una expresión. |
| NX-E0006/inferred_non_unit_result, entendido como selección por `feature_id` | `context`: primer retorno no unitario que exige anotar el resultado, por orden fuente, desde `return` hasta el final del operando. |
| NX-E0006 con `feature_id = partial_transfer` | `context`: prefijo `consume`, o nombre del intrínseco `drop`, según la construcción. |
| NX-E0104 | `previous_declaration`: nombre de la declaración anterior del mismo ámbito. |
| NX-E0105 | `declaration`: nombre declarado de la raíz inmutable. |
| NX-E0102 | `found_type`: expresión cuyo tipo incumple la regla. `expected_type`: origen fuente del tipo esperado, si existe, conforme a la regla de abajo. |
| NX-E0111/missing_argument o argument_mode_mismatch | `declaration`: nombre del parámetro requerido. |
| NX-E0111/duplicate_argument | `context`: etiqueta de la primera asociación al mismo parámetro. |
| NX-E0111/positional_after_named | `context`: etiqueta del primer argumento nombrado precedente. |
| NX-E0111/non_callable_target | `declaration`: nombre declarado del binding que ocultó o sustituyó al destino invocable. |
| NX-E0112/invalid_main_signature | `context`: firma desde `fn` hasta antes de la llave de apertura del cuerpo, sin trivia final. |
| NX-E0201 | `declaration`: nombre declarado del propietario o parámetro desde el que se solicita copiar. |
| NX-E0202 | `declaration`: raíz usada; `consumption`: sitios de consumo que alcanzan ese uso sin reinicialización; `backedge`: cierre del cuerpo de cada bucle que interviene en esos testigos. |
| NX-E1105/permission_escalation | `declaration`: parámetro que concede el permiso insuficiente. |
| NX-E1105/active_borrow_conflict o overlapping_mutable_borrows | `declaration`: raíz; `origin`: argumento completo que creó cada permiso vivo incompatible, incluido su prefijo si lo tiene. |
| NX-E1302/missing_branch_value | `context`: IfForm destinatario completo. |
| NX-E1302/invalid_value_target | `context`: llave de apertura del bloque directo que contiene el marcador inválido. Esta ubicación relacionada es una extensión de F0-0002 coherente con D13 §1.3, no una ubicación prescrita allí. |
| NX-E1302/missing_value_operand | `context`: llave de apertura de su rama destinataria; `delimiter`: salto inmediato tras `:` o, sin ese salto, token que cierra o termina el marcador vacío. |
| NX-E1302/non_final_branch_value | `context`: primera sentencia directa posterior, incluyendo su terminador si es escrito. |
| NX-E1303 | `expected_type`: operando de `value:` de la primera continuación normal; `found_type`: operando incompatible de la continuación posterior. |
| NX-E1317 | `expected_type`: tipo de resultado escrito en la firma. |
| NX-E0417/duplicate_operation o duplicate_restriction | `context`: primera ocurrencia del elemento repetido. |
| NX-E0417/invalid_operation | `context`: nombre de la función cuyo contexto no admite esa operación. |
| NX-E0412–NX-E0415 | `contract`: token exacto de la prohibición escrita. Para limpieza implícita, además `origin`: propietario, y `exit`: salida que exige esa limpieza. |
| R0102 | `context`: acceso indexado completo en lectura, destino completo en escritura o argumento completo (incluidas etiqueta y `inout`) en préstamo. El primario es el selector concreto que falla. |
| R0001/R0002 por unidad final implícita | `context`: cuerpo completo de la función. Primario: exclusivamente su `}` de cierre. |

En NX-E0102, el origen de tipo esperado es, en este orden: anotación que impone ese contexto; tipo del parámetro o resultado escrito que lo impone; expresión previa cuyo tipo debe igualarse. Si solo se exige un dominio intrínseco, no se fabrica un span para ese dominio. Para asignación, incluida la indexada, se relaciona la anotación de la raíz cuando establece el tipo esperado del elemento; si no existe, la expresión o restricción concreta que fijó ese tipo. Un inicializador divergente o indeterminado no es evidencia de un tipo concreto: se relaciona la restricción posterior que lo determinó (§4.1.1), sin inventar una anotación ni presentar el nombre del binding como anotación. Se sigue la procedencia de igualdades entre bindings hasta la restricción concreta que establece el tipo; copiar un nombre de tipo pendiente no crea una anotación. Para elementos heterogéneos de un array sin contexto se usa su primer elemento concreto; los elementos Never no lo fijan. Para igualdad o comparación entre tipos distintos se usa el operando izquierdo.

Cuando el span primario es el operador por dominio inadmisible, `found_type` señala el operando izquierdo. Para `?` señala el operando Result o no Result; cuando el defecto es el resultado no Result de la función envolvente, señala su tipo escrito, o su nombre si el resultado unidad fue omitido.

El retorno vacío causado por LF añade `delimiter` sobre el salto que terminó `return`, además del tipo esperado escrito. En L01 son `[19,20)` y `[8,11)`. En L02, el salto relacionado es `[35,36)` y la llave destinataria `[27,28)`. CRLF relaciona sus dos bytes. Esto conserva la obligación de D13 §2.6, además de los spans primarios ya publicados.

#### Selección de testigos causales

Los testigos usan identidades resueltas y el CFG fuente anterior a optimizaciones.

- Una relación de consumo incluye los sitios que pueden alcanzar el uso por un camino del CFG sin reinicializar la raíz. No se elimina una rama por tener condición constante.
- Si un testigo necesita un ciclo, basta la primera vuelta que establece la indisponibilidad. No se repiten ciclos indefinidamente en una cadena.
- Entre caminos causales alternativos se elige el de menor número de aristas; los empates se resuelven por la secuencia de offsets de sitios fuente y después por identidad. Para efectos se prefiere evidencia conocida cuando existen tanto testigos conocidos como desconocidos del mismo efecto.
- Las ubicaciones relacionadas conservan los orígenes pertinentes; `cause_chain` presenta un único testigo canónico. No pretende enumerar todas las ejecuciones.

#### `Cause.operation`

Enumeración cerrada y correspondencia:

| `operation` | `kind` | Regla que lo emite |
|---|---|---|
| `function.call` | `call` | Camino transitivo de NX-E0412–NX-E0415. |
| `intrinsic.call` | `call` | Hoja intrínseca de NX-E0412–NX-E0415. |
| `value.build_array` | `effect` | Hoja de construcción con `alloc`. |
| `value.copy` | `copy` | Hoja de copia con `alloc`. |
| `value.drop` | `drop` | Destrucción fuente o limpieza implícita en una cadena de efectos; origen de indisponibilidad en NX-E0202. |
| `resource.drop` | `drop` | Contrato de destructor confiado al final de una cadena de efectos. |
| `value.consume` | `consume` | Origen de indisponibilidad en NX-E0202. |
| `borrow.read` | `borrow` | Préstamo de lectura que sustenta NX-E1105. |
| `borrow.inout` | `borrow` | Préstamo exclusivo que sustenta NX-E1105. |
| `borrow.reborrow` | `reborrow` | Représtamo y vínculo con el permiso padre en NX-E1105. |
| `flow.backedge` | `backedge` | Retroarista del testigo de NX-E0202. |

Los valores `return` y `limit` de `Cause.kind` admitidos por la estructura general no se emiten en las cadenas de este perfil. Los presupuestos se explican mediante `data` y su origen primario; no necesitan inventar un paso causal adicional.

Reglas de construcción:

- NX-E0202 recorre su testigo desde el uso hacia el consumo: añade las retroaristas atravesadas y termina en `value.consume` o `value.drop`. El span de retroarista es `}` del cuerpo; el de consumo es la expresión `consume p` completa o la llamada intrínseca `drop(p)` completa. Entity identifica la raíz.
- NX-E1105 por conflicto comienza en el préstamo vivo incompatible de origen más temprano; los empates usan identidad. Añade sus permisos padres hasta el origen. Cada paso señala el argumento completo que creó el préstamo y su Entity de préstamo.
- NX-E1105 por elevación de permiso tiene un paso `borrow.read` o `borrow.inout` sobre el nombre del parámetro que concede el permiso, con Entity de parámetro. No inventa un LoanId.
- NX-E0412–NX-E0415 incluyen cada sitio de llamada desde el cuerpo comprobado hasta la hoja. Los spans de pasos con kind `call` son los nombres de destino. Una hoja local de construcción o copia ocupa la expresión completa. Para destrucción explícita, el primario señala el nombre `drop` y la hoja `value.drop`, de kind `drop`, ocupa la llamada completa sin terminador; después aparece `resource.drop` cuando corresponde. Una limpieza implícita comienza en el propietario y termina en `resource.drop`.
- La hoja `resource.drop` usa `span = null` y Entity de tipo `MockResource` o `MockBlockingResource`; el manifiesto confiado no tiene un span en el archivo del usuario.
- Los pasos de llamadas intermedias tienen Entity de función destino; una hoja intrínseca tiene `prelude:<nombre>`. Construcción usa Entity de tipo; copia y destrucción de un binding usan su Entity de binding o parámetro.
- `evidence = verified` identifica hechos estáticos del modelo y contratos del manifiesto confiado. La hoja de `mock_unknown` usa `unknown`. No se emite `external_assumption` en este perfil sin interfaces externas.
- Las demás reglas, incluidos los siete R, tienen `cause_chain = []`. No se agrega un stack trace variable como parte de ese campo.

Para E04, la cadena exacta por operaciones, spans y entidades es:

| Orden | `kind` / `operation` | Span | Entity |
|---:|---|---|---|
| 1 | `call` / `function.call` | `[62,67)` | `function`, `function:24` |
| 2 | `call` / `function.call` | `[32,36)` | `function`, `function:3` |
| 3 | `call` / `intrinsic.call` | `[10,15)` | `intrinsic`, `prelude:print` |

Los tres pasos llevan `evidence = verified`. El diagnóstico tiene `operation = function.call`, primario `[62,67)` y contrato `no_io` en `[56,61)`.

Una limpieza que pueda ejecutarse en varias salidas elige para el diagnóstico la primera salida por offset fuente; un empate usa el orden normal, return, propagación y fallo de las salidas del CFG. Esto fija `exit` sin depender del orden de una tabla hash.

#### Identificadores de reparación

El vocabulario de `Repair.id` emitido por la interfaz ordinaria de este perfil contiene únicamente:

```text
insert_terminator
```

Se emite para NX-E0005, con una edición que inserta `;` al final de la expresión precedente, o del introductor cuando la sentencia no tiene expresión. Esa posición precede a la trivia que separa la sentencia del token inesperado. En el ejemplo de §13.7 coincide con byte 17.

La interfaz ordinaria emite esa edición como `proposed`, `validation = null`. No ejecuta una segunda compilación oculta para decidir qué JSON devolver. Las demás reglas tienen `repair = []`; sus positivos pareados y alternativas explicadas no se convierten automáticamente en ediciones estructuradas.

Una operación explícita de recomprobación puede producir la misma reparación con Validation y estado `applicable` solo bajo §13.6. El arnés prueba por separado esa recomprobación. En particular, insertar `;` puede seguir dejando un retorno o valor de rama ausente; entonces la reparación no es aplicable.

Las alternativas de D13 —introducir `return`, introducir `value:` o mover `(` antes del salto— siguen siendo decisiones descritas en la explicación y en sus positivos. No se anuncian como ediciones automáticas equivalentes. Añadir otro ID emitible requiere ampliar este vocabulario y publicar su construcción exacta.


### 13.17. Componentes no resueltos y dominios de tipos (13.16 retirada)

#### `unresolved_components`

NX-E0113 enumera rutas dentro del **tipo de resultado del constructor señalado**, después de aplicar todo el contexto local disponible.

La gramática cerrada de una ruta es:

```text
ComponentPath = "$" { ".element" | ".ok" | ".err" }
```

`$` es el tipo raíz; `.element` entra en Array, `.ok` y `.err` en los parámetros de Result. Se emite una ruta por posición de tipo todavía indeterminada, sin identificadores internos de variables del solver. Una misma incógnita presente en dos posiciones produce ambas rutas.

El recorrido es preorden, con elemento de Array y con `.ok` antes de `.err` en Result; no se ordena lexicográficamente después. No hay duplicados de la misma ruta.

Ejemplos:

| Constructor sin contexto suficiente | `unresolved_components` |
|---|---|
| `Ok(1)` | `["$.err"]` |
| `Err(7)` | `["$.ok"]` |
| `Ok(Ok(1))` | `["$.ok.err","$.err"]` |
| `Ok(1)` con tipo esperado `Result<i64,i64>` | No emite NX-E0113. |

Un array vacío cuyo elemento no puede determinarse conserva NX-E0103. No se emite además NX-E0113 por una indeterminación del constructor exterior que dependa de ese array inválido.

#### `expected_type` y `found_type`

Un tipo concreto se escribe sin espacios:

```text
i64
bool
()
MockResource
MockBlockingResource
Array<T>
Result<T,E>
```

`Never` es exclusivamente el fondo interno de ausencia de continuación (§4.1.1), distinto de variable por inferir y contaminación. Nunca aparece en `data`, `expected_type`, `found_type` ni como tipo escribible. Tampoco se serializan TypeId, variables como `?17` ni nombres dependientes de interning.

Los únicos dominios adicionales permitidos en `expected_type` son:

```text
domain:copyable_data
domain:array
domain:mock_resource
domain:result
```

`found_type` siempre contiene el tipo concreto encontrado, no una descripción libre.

| Comprobación | `expected_type` | `found_type` |
|---|---|---|
| `+` o `-`, primer operando incompatible de izquierda a derecha | `i64` | Tipo de ese operando. |
| `<` o `==`, operandos de tipos distintos | Tipo del izquierdo | Tipo del derecho. |
| `<`, tipos iguales pero inadmisibles | `i64` | Tipo común. |
| `==`, tipos iguales pero no copiables | `domain:copyable_data` | Tipo común. |
| `assert_eq`, tipos distintos | Tipo del primer argumento | Tipo del segundo. |
| `assert_eq`, mismo tipo fuera de su dominio | `domain:copyable_data` | Tipo común. |
| `print` sobre un tipo fuera de su dominio | `domain:copyable_data` | Tipo del argumento. |
| NX-E0102/not_indexable | `domain:array` | Tipo de la base. |
| Índice de Array no entero | `i64` | Tipo del índice. |
| `inspect` sin recurso | `domain:mock_resource` | Tipo del argumento. |
| `mock_read_records` con argumento incompatible | `MockResource` | Tipo del argumento. |
| `?` sobre operando no Result | `domain:result` | Tipo del operando. |
| `?` dentro de función cuyo resultado no es Result | `domain:result` | Resultado de la función, incluido `()` implícito. |
| `?` con tipos de error incompatibles | Tipo E del resultado de la función | Tipo E del operando. |
| NX-E1303 | Tipo de la primera continuación normal | Tipo de la continuación incompatible. |
| Restantes contextos anotados, parámetros, retornos, asignaciones o condiciones | Tipo concreto exigido por ese contexto | Tipo de la expresión comprobada. |

En `?` se comprueba primero que el operando sea Result, después el resultado de la función y después la igualdad de errores. Estas precisiones conservan los primarios R01–R03.

S16 exige `i64`/`bool`; S17, `i64`/`Array<i64>`; S18, `i64`/`bool`; S19, `domain:copyable_data`/`MockResource`; S20, `i64`/`Result<i64,i64>`; S21, `domain:copyable_data`/`Result<MockResource,i64>`.
## 14. Corpus mínimo con causa y ubicación

Todas las filas siguientes son **expectativas de diseño, NO EVALUADAS**. Las etiquetas locales de fixtures de este ADR, incluidas las copias migradas K01/K02, tienen identidad canónica `F2-` seguida de la etiqueta: por ejemplo, `F2-S03`, `F2-E01` y `F2-L01`. Dentro de F0-0002 se permite omitir ese prefijo; fuera del ADR es obligatorio al citar estas fixtures. Las referencias explícitas a ejemplos de la base u otros ADR, como EX01–EX04, conservan la identidad de su documento de origen.

Convención: `⟦texto⟧` marca exactamente el span primario. El generador elimina únicamente los delimitadores antes de compilar; los offsets esperados son los bytes UTF-8 del texto marcado en la fuente resultante. No son tokens Nexo. En positivos independientes, el marcador identifica la operación que se quiere observar, sin esperar diagnóstico.

Cada positivo pareado se obtiene aplicando la edición indicada sobre la fuente completa, ya sin marcadores. Debe completar comprobación; no basta superar el parser. Las sustituciones de texto marcado afectan solo a esa ocurrencia.

### 14.1. Funciones, tipos y acceso

| ID | Fuente negativa | Código/subcausa | Positivo pareado |
|---|---|---|---|
| S01 | `fn f(x:i64){} fn main(){⟦f⟧();}` | `NX-E0111/missing_argument` | `f(1);`. |
| S02 | `fn f(x:i64){} fn main(){f(1,⟦2⟧);}` | `NX-E0111/excess_positional_argument` | Quitar `,2`. |
| S03 | `fn f(x:i64){} fn main(){f(⟦y⟧:1);}` | `NX-E0111/unknown_argument_label` | Cambiar `y` por `x`. |
| S04 | `fn f(x:i64){} fn main(){f(x:1,⟦x⟧:2);}` | `NX-E0111/duplicate_argument` | Quitar `,x:2`. |
| S05 | `fn f(x:i64,y:i64){} fn main(){f(x:1,⟦2⟧);}` | `NX-E0111/positional_after_named` | Escribir `y:2`. |
| S06 | `fn f(inout x:i64){} fn main(){var x=1;f(⟦x⟧);}` | `NX-E0111/argument_mode_mismatch` | Escribir `inout x`. |
| S07 | `fn main(){let f=1;⟦f⟧();}` | `NX-E0111/non_callable_target` | Quitar `let f=1;` y añadir `fn f(){}` al módulo. |
| S08 | `fn f(inout x:i64){} fn main(){let x=1;f(inout ⟦x⟧);}` | `NX-E0105` | Cambiar `let x` por `var x`. |
| S09 | `fn main(){let r=⟦Ok(1)⟧;}` | `NX-E0113` | Anotar `r:Result<i64,i64>`. |
| S10 | `fn main(){let xs=⟦[]⟧;}` | `NX-E0103` | Anotar `xs:Array<i64>`. |
| S11 | `fn f()->i64 { ⟦}⟧ fn main(){}` | `NX-E1317` | Insertar `return 1;` antes del cierre marcado. |
| S12 | `fn f()->i64{return ⟦true⟧;} fn main(){}` | `NX-E0102/type_mismatch` | Cambiar el operando por `1`. |
| S13 | `fn f(){} fn ⟦f⟧(){} fn main(){}` | `NX-E0104`, `declaration_kind = function` | Renombrar la segunda función a `g`. |
| S14 | `fn f(x:i64,⟦x⟧:i64){} fn main(){}` | `NX-E0104`, `declaration_kind = parameter` | Renombrar el segundo parámetro a `y`. |
| S15 | `fn main(){⟦ausente⟧();}` | `NX-E0101`, `use_kind = call_target` | Añadir `fn ausente(){}` al módulo. |
| S16 | `fn main(){let b=true⟦<⟧false;}` | `NX-E0102/type_mismatch` | Sustituir los operandos por `0` y `1`. |
| S17 | `fn main(){let b=[1]⟦<⟧[2];}` | `NX-E0102/type_mismatch` | Sustituir `<` por `==`. |
| S18 | `fn main(){let b=1==⟦true⟧;}` | `NX-E0102/type_mismatch` | Sustituir `true` por `1`. |
| S19 | `fn main(){let r=mock_open(1);let b=r⟦==⟧r;}` | `NX-E0102/type_mismatch` | Sustituir `let b=r==r;` por `inspect(r);`. |
| S20 | `fn main(){let a:Result<i64,i64>=Ok(1);let b=a⟦<⟧a;}` | `NX-E0102/type_mismatch` | Sustituir `<` por `==`. |
| S21 | `fn main(){let a:Result<MockResource,i64>=Ok(mock_open(1));let b=a⟦==⟧a;}` | `NX-E0102/type_mismatch` | Sustituir `let b=a==a;` por `drop(a);`. |
| S22 | `fn main(){let ⟦match⟧=1;}` | `NX-E0002` | Sustituir el nombre por `match_value`. |

S22 exige etapa `parse`, token `match` en `[14,19)`, `parser_context = binding_name`, `expected_tokens = ["Name"]`, `expected_tokens_kind = admissible_set` y `recognized_prefix = [10,14)`, desde `let` hasta el token inesperado, incluida la trivia intermedia.

Para `run` se añaden: módulo `fn f(){}` con span vacío EOF y `NX-E0112/missing_main`; y `fn ⟦main⟧(x:i64){}` con `NX-E0112/invalid_main_signature`. Los positivos añaden `fn main(){}` y quitan el parámetro, respectivamente.

Positivo de igualdad estructural:

```nexo
fn main(){
    let a:Result<Array<i64>,i64>=Ok([1,2]);
    let b:Result<Array<i64>,i64>=Ok([1,2]);
    let c:Result<Array<i64>,i64>=Err(7);
    assert_eq(a==b,true);
    assert_eq(a==c,false);
    assert_eq([1]==[1,2],false);
    assert_eq(()==(),true);
}
```

Debe comprobarse sin copias de operandos de `==`; no se acredita esa propiedad contando solo su resultado booleano.

### 14.2. Propiedad y préstamos

| ID | Fuente negativa | Código/subcausa | Positivo pareado |
|---|---|---|---|
| P01 | `fn main(){let r=mock_open(1);let s=⟦r⟧;}` | `NX-E0201` | `let s=consume r;`. |
| P02 | `fn main(){let r=mock_open(1);let s=consume r;inspect(⟦r⟧);}` | `NX-E0202/definitely_consumed` | Inspeccionar `s`. |
| P03 | `fn main(){let x=1;drop(x);print(⟦x⟧);}` | `NX-E0202/definitely_consumed` | Mover `print(x);` antes de `drop(x);`. |
| P04 | `fn f(x:MockResource)->MockResource{return consume ⟦x⟧;} fn main(){}` | `NX-E1105/permission_escalation` | Declarar `consume x:MockResource`. |
| P05 | `fn f(inout x:i64,inout y:i64){} fn main(){var x=1;f(inout x,inout ⟦x⟧);}` | `NX-E1105/overlapping_mutable_borrows` | Declarar `var y=2;` antes de la llamada y usar `inout y` como segundo argumento. |
| P06 | `fn f(x:i64,inout y:i64){} fn main(){var x=1;f(x,inout ⟦x⟧);}` | `NX-E1105/active_borrow_conflict` | Primer argumento `x+0`, temporal independiente. |
| P07 | `fn f(inout x:i64,inout y:i64){} fn main(){var a=[1,2];f(inout a[0],inout ⟦a[1]⟧);}` | `NX-E1105/overlapping_mutable_borrows` | Añadir `var b=[3,4];` antes de la llamada y sustituir el lugar marcado por `b[1]`. |
| P08 | `fn f(b:bool){let r=mock_open(1);if b{drop(r);}inspect(⟦r⟧);} fn main(){}` | `NX-E0202/maybe_consumed` | Añadir `return;` después de `drop(r);` en la rama. |
| P09 | `fn f(b:bool){let r=mock_open(1);while b{drop(⟦r⟧);}} fn main(){}` | `NX-E0202/maybe_consumed` | Mover `let r=mock_open(1);` al comienzo del cuerpo del bucle. |
| P10 | `fn main(){let r:Result<MockResource,i64>=Ok(mock_open(1));let s=⟦r⟧;drop(s);}` | `NX-E0201` | Sustituir el inicializador marcado por `consume r`. |
| P11 | `fn main(){let r:Result<i64,MockResource>=Ok(1);let s=⟦r⟧;}` | `NX-E0201` | Sustituir el inicializador marcado por `consume r`. |

P09 debe identificar la retroarista y el consumo anterior. P11 demuestra que conocer la variante `Ok` no vuelve copiable un tipo cuyo `Err` contiene recurso.

Los positivos adicionales tienen fuente y operación observada:

| ID | Fuente positiva | Obligación |
|---|---|---|
| PP01 | `fn leer(r:MockResource){inspect(r);inspect(r);} fn main(){let r=mock_open(1);⟦leer(r)⟧;inspect(r);}` | Lectura repetida prestada: `open:0:1`, tres `inspect:0:1`, un `close:0:1`. |
| PP02 | `fn inc(inout x:i64){x=x+1;} fn dos(inout x:i64){inc(inout x);⟦inc(inout x)⟧;} fn main(){var x=0;dos(inout x);assert_eq(x,2);}` | El primer représtamo termina antes del segundo; resultado 2. |
| PP03 | `fn copia(a:Array<i64>)->Array<i64>{return a;} fn main(){var a=[1];let b=⟦copia(a)⟧;a[0]=2;assert_eq(b,[1]);}` | El retorno copia datos del parámetro prestado; modificar el original no modifica `b`. |
| PP04 | `fn main(){var r=mock_open(1);drop(r);⟦r=mock_open(2)⟧;inspect(r);}` | Reinicialización completa: `open:0:1`, `close:0:1`, `open:1:2`, `inspect:1:2`, `close:1:2`. |
| PP05 | `fn main(){let r:Result<MockResource,i64>=Ok(mock_open(1));let s=⟦consume r⟧;drop(s);}` | Transferencia del mismo tipo rechazado por copia en P10; una adquisición y un cierre. |

### 14.3. `Result`, ramas y terminadores

| ID | Fuente negativa | Código/subcausa | Positivo pareado |
|---|---|---|---|
| R01 | `fn main()->Result<(),i64>{let x=1⟦?⟧;return Ok(());}` | `NX-E0102/type_mismatch` | Operando `try_add(1,0)`. |
| R02 | `fn main(){let x=try_add(1,2)⟦?⟧;}` | `NX-E0102/type_mismatch` | Anotar resultado `Result<(),i64>` y añadir `return Ok(());`. |
| R03 | `fn main()->Result<(),bool>{let x=try_add(1,2)⟦?⟧;return Ok(());}` | `NX-E0102/type_mismatch` | Cambiar el tipo de error a `i64`. |
| B01 | `fn f(b:bool)->i64{return ⟦if⟧ b{value:1;};} fn main(){}` | `NX-E1302/missing_else` | Añadir `else{value:2;}`. |
| B02 | `fn f()->i64{return if true{1;⟦}⟧else{value:2;};} fn main(){}` | `NX-E1302/missing_branch_value` | Cambiar `1;` por `value:1;`. |
| B03 | `fn f(){let n=if true{value:1;}else{value:⟦true⟧;};} fn main(){}` | `NX-E1303` | Cambiar el valor marcado por `2`. |
| B04 | `fn main(){⟦value:⟧1;}` | `NX-E1302/invalid_value_target` | Sustituir la sentencia por `let value=1;`. |
| B05 | `fn f()->i64{return if true{⟦value:⟧}else{value:2;};} fn main(){}` | `NX-E1302/missing_value_operand` | Insertar `1;` tras el marcador. |
| B06 | `fn f()->i64{return if true{⟦value:⟧1;2;}else{value:2;};} fn main(){}` | `NX-E1302/non_final_branch_value` | Quitar `2;` de la primera rama. |
| B07 | `fn f()->i64{return 1 ⟦}⟧ fn main(){}` | `NX-E0005` | Insertar `;` después de `1`. |
| B08 | `fn f()->i64{return if true{value:1 ⟦}⟧else{value:2;};} fn main(){}` | `NX-E0005` | Insertar `;` después del primer `1`. |
| B09 | `fn f()->i64{return if true{value:1;}else{value:⟦true⟧;};} fn main(){}` | `NX-E0102/type_mismatch` | Cambiar el valor marcado por `2`. |

B03 no recibe tipo esperado: compara los tipos sintetizados de las continuaciones. B09 recibe `i64` desde `return`: su valor booleano incumple ese contexto. No se acepta intercambiar ambos códigos.

B01/B02/B04–B06 prueban estructura, con tipos de los operandos presentes compatibles. B07/B08 prueban terminadores y exigen token `}`, prefijo reconocido y `expected_tokens_kind = canonical_insertion`.

Se añaden fuentes con LF real para:

- `return` seguido de `(1);`: `NX-E0102/type_mismatch` en `return`; positivo con `return (` antes del salto.
- `value:` seguido de `(1);`: `missing_value_operand` en el introductor; positivo con apertura antes del salto.
- `value` seguido de `(x)` en otra línea: llamada al identificador, cuando su resolución lo permita.
- `let value=...`, `value -1` y `value - 1`: mismo significado léxico pertinente, sin reservar `value`.
- `value: ();`: entrega explícita de unidad.
- Expresión final terminada por LF: descarte, seguido de `NX-E1317` si falta retorno.
- CRLF, comentarios Unicode y EOF sin salto: mismos árboles y spans sobre sus bytes reales.

### 14.4. Efectos

| ID | Fuente negativa | Código y causa | Positivo pareado |
|---|---|---|---|
| E01 | `fn main() with no_alloc{let a=⟦[1]⟧;}` | `NX-E0412`, conocida, construcción | Cambiar el array por un escalar. |
| E02 | `fn copia(a:Array<i64>)->Array<i64> with no_alloc{return ⟦a⟧;} fn main(){}` | `NX-E0412`, conocida, copia | Cambiar a resultado `i64` y `return len(a);`. |
| E03 | `fn main() with no_block{let ⟦r⟧=mock_blocking_open(1);}` | `NX-E0413`, conocida, limpieza | Usar `mock_open(1)`. |
| E04 | `fn hoja(){print(1);} fn medio(){hoja();} fn main() with no_io{⟦medio⟧();}` | `NX-E0415`, cadena `main → medio → hoja → print` | Quitar `print(1);` de `hoja`. |
| E05 | `fn main() with no_suspend{⟦mock_unknown⟧();}` | `NX-E0414`, desconocida | Sustituir la llamada por `assert_eq(1,1);`. |
| E06 | `fn main() with no_io{if false{⟦print⟧(1);}}` | `NX-E0415`, rama conservada | Sustituir `print(1);` por `assert_eq(1,1);`. |

E05 se instancia también con `no_alloc`, `no_block` y `no_io`, esperando respectivamente `NX-E0412`, `NX-E0413` y `NX-E0415`, siempre con causa desconocida.

Pares concretos de estructura de contratos:

| ID | Fuente negativa | Código/subcausa | Positivo pareado |
|---|---|---|---|
| W01 | `fn main() with {call(no_io),⟦call⟧(no_alloc)}{}` | `NX-E0417/duplicate_operation` | Quitar `,call(no_alloc)`. |
| W02 | `fn main() with {⟦resume⟧(no_io)}{}` | `NX-E0417/invalid_operation` | Cambiar `resume` por `call`. |
| W03 | `fn main() with no_io,⟦no_io⟧{}` | `NX-E0417/duplicate_restriction` | Quitar la segunda restricción y su coma. |
| W04 | `fn main() with ⟦{}⟧{}` | `NX-E0417/empty_operation_list` | Sustituir el mapa por `{call(unrestricted)}`. |
| W05 | `fn main() with ⟦unrestricted,no_io⟧{}` | `NX-E0417/unrestricted_mixed` | Quitar `unrestricted,`. |
| W06 | `fn main() with {call(⟦no_io,unrestricted⟧)}{}` | `NX-E0417/unrestricted_mixed` | Quitar `,unrestricted`. |
| W07 | `fn main() with unrestricted,⟦unrestricted⟧,no_io{}` | `NX-E0417/duplicate_restriction` | Sustituir la lista completa por `no_io`. |

W02 precede a una ausencia derivada de `call`; W04 precede a `missing_body_operation`; W07 comprueba que repetición precede a mezcla.

Las subcausas `missing_with_clause` y `operation_list_required` requieren cotas invocables diferidas. `missing_body_operation` no tiene una emisión aislada en una función ordinaria de este perfil: mapa vacío e inclusión de otra operación tienen causas prioritarias específicas. No se inventan fixtures de esas subcausas para incrementar cobertura.

### 14.5. Ejecución y limpieza

Los programas siguientes deben comprobarse antes de ejecutarse.

1. **Fallo al preparar argumentos.**

   ```nexo
   fn f(consume r:MockResource,n:i64){}
   fn main(){f(mock_open(1),9223372036854775807+1);}
   ```

   Esperado: `R0101`; eventos `open:0:1`, `close:0:1`; el cuerpo de `f` no se ejecuta. Span del desbordamiento: la suma completa.

2. **Propagación entre frames.**

   ```nexo
   fn fallo()->Result<i64,i64>{
       let r=mock_open(2);
       return Err(9);
   }
   fn main()->Result<(),i64>{
       let r=mock_open(1);
       let n=fallo()?;
       print(n);
       return Ok(());
   }
   ```

   Esperado: `Err(9)`, salida de proceso 1, sin salida de programa ni diagnóstico R; eventos `open:0:1`, `open:1:2`, `close:1:2`, `close:0:1`.

3. **Recursos dentro de `Result`.**

   ```nexo
   fn crear()->Result<MockResource,i64>{
       return Ok(mock_open(3));
   }
   fn main()->Result<(),i64>{
       let r=crear()?;
       inspect(r);
       return Ok(());
   }
   ```

   Esperado: `open:0:3`, `inspect:0:3`, `close:0:3`. El payload no se cierra al retornar de `crear`.

4. **Reasignación completa.**

   ```nexo
   fn main(){
       var r=mock_open(1);
       r=mock_open(2);
   }
   ```

   Esperado: `open:0:1`, `open:1:2`, `close:0:1`, `close:1:2`.

5. **Precedencia de errores heredada.**

   ```nexo
   fn main(){
       let r=mock_open(1);
       var a=[0];
       a[-1]=9223372036854775807+1;
   }
   ```

   Esperado: `R0101`, no `R0102`, y cierre único. Span: suma completa del RHS.

6. **Argumentos nombrados.**

   ```nexo
   fn marca(n:i64)->i64{print(n);return n;}
   fn f(a:i64,b:i64){}
   fn main(){f(b:marca(2),a:marca(1));}
   ```

   Esperado: salida `["2","1"]`, sin eventos. Se conserva el orden escrito aunque difiera del orden de parámetros.

7. **Cuotas.**

   Se ejecutan las cuatro recetas de §11.3 y las fronteras de §11.5. Cada fila exige su propia causa: `R0001` para trabajo; `R0002` con `runtime_budget_kind` correcto para frames, celdas y observaciones. Se compara perfil, límite, ocupación propuesta, span desencadenante y limpieza. El crecimiento por copias solo acredita la cuota de celdas.

8. **Entorno mock y bytes.**

   El ejemplo literal JSON de §12.4 debe producir exactamente 27 bytes virtuales y salida 0. Su variante de escritura fallida debe producir solo el evento de error y salida 1. Se ejercitan colas agotadas, lectura vacía, error de registro intermedio, ambas rutas y errores de respuesta. Las fronteras de observación incluyen prefijos, LF y reservas de cierre; el tamaño del JSON de transporte no sustituye esa medida.

9. **RT01 — Desbordamiento de resta.**

   Fuente: `fn main(){print(⟦-9223372036854775808-1⟧);}`. Esperado: `R0103`, namespace `F0-0002-sim`, `stage = runtime`, `operation = value.sub`, span `[16,38)`, `data = {}`, sin salida de programa ni eventos y salida de proceso 1. El positivo sustituye únicamente el sustraendo final `1` por `0`: imprime `-9223372036854775808`, retorna unidad y termina con salida 0.

### 14.6. Cobertura de alcance por `feature_id`

Todas estas filas esperan `code = NX-E0006` y `subcause = null`. El campo de la tercera columna es `feature_id`, no subcausa. Cada fila alcanza únicamente una exclusión pertinente antes de cualquier error semántico derivado.

| ID | Fuente negativa | `feature_id` | Positivo pareado |
|---|---|---|---|
| F01 | `fn main(){let r=mock_open(1);let s=⟦move⟧ r;}` | `legacy_move` | Cambiar `move` por `consume`. |
| F02 | `fn ⟦f⟧(){return 1;} fn main(){}` | `inferred_non_unit_result` | Añadir `->i64` después de `f()`. |
| F03 | `fn main(){let x=⟦0.5⟧;}` | `float_literal` | Sustituir el literal por `0`. |
| F04 | `fn main(){let n=1;let x=0⟦..⟧n;}` | `range` | Sustituir `..` por `+`. |
| F05 | `fn main(){let x=(1⟦,⟧2);}` | `tuple` | Quitar `,2`; queda agrupación `(1)`. |
| F06 | `⟦@⟧derive(JsonEncode) fn main(){}` | `attributes` | Quitar `@derive(JsonEncode)`. |
| F07 | `fn f(x:i64⟦=⟧1){} fn main(){f(1);}` | `default_argument` | Quitar `=1`. |
| F08 | `fn main(){let f=⟦fn⟧[](){return ();};}` | `closure` | Sustituir el inicializador completo por `()`. |
| F09 | `fn f(){} fn main(){let g=⟦f⟧;}` | `function_value` | Sustituir `let g=f;` por `f();`. |
| F10 | `fn f(){} fn main(){⟦consume⟧ f();}` | `receiver_mode` | Quitar el prefijo `consume`. |
| F11 | `fn main(){let a=[1];let x=consume ⟦a[0]⟧;}` | `partial_transfer` | Quitar `consume`; se copia el elemento. |
| F12 | `fn main(){let a=⟦[mock_open(1)]⟧;}` | `resource_array` | Quitar los corchetes del inicializador. |
| F13 | `fn main() with ⟦no_heap_alloc⟧{}` | `no_heap_alloc` | Sustituir por `no_alloc`. |
| F14 | `fn main(){⟦match⟧ true{true=>{},false=>{}}}` | `match` | Sustituir el `match` completo por `if true{}else{}`. |
| F15 | `fn f(x:⟦Option⟧<i64>){} fn main(){}` | `option` | Sustituir `Option<i64>` por `Result<i64,i64>`. |
| F16 | `⟦struct⟧ S{} fn main(){}` | `user_types` | Quitar `struct S{}`. |
| F17 | `⟦trait⟧ T{} fn main(){}` | `traits_generics` | Quitar `trait T{}`. |
| F18 | `⟦use⟧ std.app;fn main(){}` | `modules_visibility` | Quitar `use std.app;`. |
| F19 | `⟦extend⟧ i64{} fn main(){}` | `extensions` | Quitar `extend i64{}`. |
| F20 | `fn f(x:i64)->i64 ⟦from⟧(x){return x;} fn main(){}` | `provenance_clauses` | Quitar `from(x)`. |
| F21 | `⟦const⟧ X:i64=comptime 1;fn main(){}` | `comptime` | Quitar la declaración de constante completa. |
| F22 | `fn main(){let x=⟦"hola"⟧;}` | `text` | Sustituir la cadena por `1`. |
| F23 | `fn main(){let a=[1];a⟦.⟧foo();}` | `methods` | Sustituir `a.foo()` por `len(a)`. |
| F24 | `fn main(){var x=1;x⟦+=⟧1;}` | `compound_assignment` | Sustituir `x+=1;` por `x=x+1;`. |
| F25 | `fn main(){⟦for⟧ n in [1]{}}` | `for_iteration` | Sustituir el bucle completo por `while false{}`. |
| F26 | `fn main(){while false{⟦break⟧;}}` | `loop_control` | Quitar `break;`. |
| F27 | `⟦gen⟧ fn g(){} fn main(){}` | `generator` | Quitar `gen`. |
| F28 | `⟦async⟧ fn f(){} fn main(){}` | `async` | Quitar `async`. |
| F29 | `fn f(x:⟦u32⟧){} fn main(){}` | `unsupported_type` | Sustituir `u32` por `i64`. |
| F30 | `fn f(x:⟦String⟧){} fn main(){}` | `unsupported_type` | Sustituir `String` por `Array<i64>`. |
| F31 | `fn f(x:⟦app.Error⟧){} fn main(){}` | `unsupported_type` | Sustituir la ruta completa por `i64`. |
| F32 | `fn main(){⟦region⟧ r{}}` | `region` | Quitar `region r`; queda un bloque ordinario. |
| F33 | `fn main(){let x=⟦self⟧;}` | `methods` | Sustituir `self` por `0`. |

Las etapas esperadas son `parse`, excepto F02/F12/F15/F29–F31, que exigen reconocimiento en tipos, y F09, que exige resolución de nombres. El literal decimal y la cadena se tokenizan antes de que el parser emita su exclusión.

En F02 el span es el nombre de la función cuya omisión de resultado debe repararse; el retorno no unitario es ubicación relacionada. En F12 es todo el array, porque la exclusión depende de su tipo. En F11 es la proyección consumida y se relaciona el prefijo.

No hay fila emitible `indirect_call`: se retira del perfil por la razón de §3.4. Los positivos de esta tabla prueban el diagnóstico de alcance y su reparación; no acreditan semántica de la característica diferida.


### 14.7. Cobertura adicional de nombres, preámbulo y operaciones

Estas filas completan §14.1 y conservan la convención de marcadores.

| ID | Fuente negativa | Esperado | Positivo pareado |
|---|---|---|---|
| S23 | `fn main(){let x=1;let ⟦x⟧=2;}` | NX-E0104; `data.declaration_kind = binding`; `[22,23)` | Renombrar el segundo binding a `y`. |
| S24 | `fn f(x:i64){let ⟦x⟧=1;} fn main(){}` | NX-E0104; `data.declaration_kind = binding`; `[16,17)`; parámetro original relacionado | Encerrar `let x=1;` en un bloque hijo. |
| S25 | `fn main(){let x=⟦1⟧[0];}` | NX-E0102/not_indexable; `[16,17)` | Sustituir la base marcada por `[1]`. |
| S26 | `fn main(){let x=1;inspect(⟦x⟧);}` | NX-E0102/resource_required; `[26,27)` | Sustituir el inicializador de `x` por `mock_open(1)`. |
| S27 | `fn main(){let x=1⟦*⟧2;}` | NX-E0002; `[17,18)` | Sustituir `*` por `+`. |
| S28 | `fn main(){let x=1⟦!=⟧2;}` | NX-E0002; `[17,19)` | Sustituir `!=` por `==`. |

S23/S24 son de etapa `names`; S25/S26, de `types`; S27/S28, de `parse`.

LX01–LX05 materializan, respectivamente, `/`, `%`, `!`, `&` y `|` como el único carácter marcado de `fn main(){let x=1⟦/⟧2;}`. Cada negativo exige NX-E0001, `stage = lex`, `operation = syntax.lex`, span `[17,18)`, token de kind `invalid` cuyo texto es ese carácter, `parser_context = lexical_token`, `expected_tokens = ["valid_token"]`, `expected_tokens_kind = lexical_category` y `recognized_prefix = [0,17)`. El positivo de cada fila sustituye ese carácter por `+`. Las reglas de §13.14 determinan los demás campos.

En S27/S28 se exige `parser_context = unsupported_binary_operator`, `recognized_prefix = [10,17)`, los tokens esperados de §3.5 y `unexpected_token.kind = symbol`. No se admite NX-E0005 ni un fallo léxico anterior.

Positivo PP06 de sombreado del preámbulo:

```nexo
fn print(x:i64)->i64{return x+1;}
fn main() with no_io{
    let n=⟦print(2)⟧;
    assert_eq(n,3);
}
```

Se elimina el marcador antes de compilar. La llamada resuelve la función de usuario, retorna 3 y no emite salida ni eventos. El contrato `no_io` debe aceptarse. No se aplican por comparación textual los efectos o el comportamiento del intrínseco `print`.

### 14.8. Fuentes completas para las reglas de LF

Estas fixtures concretan las viñetas de §14.3. Los bloques contienen **LF reales**; sus fuentes se obtienen quitando únicamente los marcadores y sin añadir LF después de la última línea. Ningún resultado queda condicionado a «cuando su resolución lo permita».

#### L01 — Salto después de `return`

```nexo
fn f()->i64{
⟦return⟧
(1);
}
fn main(){}
```

Esperado: NX-E0102/type_mismatch, etapa `types`, span `[13,19)`. El retorno vacío es incompatible con `i64`.

Positivo: sustituir exactamente `return` + LF + `(1);` por `return (` + LF + `1);`. La agrupación se abre antes del salto.

#### L02 — Salto después de `value:`

```nexo
fn f()->i64{
return if true{
⟦value:⟧
(1);
}else{value:2;};
}
fn main(){}
```

Esperado: NX-E1302/missing_value_operand, etapa `parse`, span `[29,35)`. Tiene prioridad sobre `non_final_branch_value` y errores derivados del marcador vacío.

Positivo: sustituir `value:` + LF + `(1);` por `value: (` + LF + `1);`.

#### L03 — Nombre `value` seguido de llamada en otra línea

```nexo
fn value(x:i64)->i64{return x;}
fn main(){let x=7;let y=⟦value
(x)⟧;assert_eq(y,7);}
```

Positivo: comprobación y ejecución satisfactorias, sin salida ni eventos. La operación observada ocupa `[56,65)`. El nombre resuelve explícitamente la función declarada; `(` continúa su llamada.

#### L04 — Identificador ordinario y resta

```nexo
fn main(){
let value=2
let a=⟦value -1⟧
let b=value - 1
assert_eq(a,b)
}
```

Positivo: resultado unidad; ambos valores son 1. La operación marcada ocupa `[29,37)`. Sustituir solo `value -1` por `value - 1` conserva el árbol normalizado y el trabajo; cambia su span a `[29,38)`.

Los saltos insertan los terminadores de bindings y de la llamada; `value` no se reserva.

#### L05 — Entrega explícita de unidad

```nexo
fn main(){
let x=if true{⟦value: ();⟧}else{value: ();};
assert_eq(x,());
}
```

Positivo: comprobación y ejecución satisfactorias. El marcador de operación ocupa `[25,35)` y contiene un operando unidad explícito.

#### L06 — Expresión final descartada

```nexo
fn f()->i64{
1
⟦}⟧
fn main(){}
```

Esperado: NX-E1317, etapa `ownership`, span `[15,16)`. El LF termina una sentencia descartada; no produce retorno implícito.

Positivo: sustituir la línea `1` por `return 1`.

#### L07 — Unicode, CRLF y EOF sin salto

```nexo
// á🙂
fn main(){
let x=⟦missing⟧;
}
```

Esperado: NX-E0101, etapa `names`, `data.use_kind = value`, línea 3 y columna 7.

| Variante materializada | Span |
|---|---|
| LF, sin salto final | `[27,34)` |
| Reemplazar cada LF por CRLF, sin salto final | `[29,36)` |
| Añadir un LF final a la primera variante | `[27,34)` |

Positivo pareado: insertar `let missing=1;` inmediatamente antes de `let x=missing;`. Debe comprobarse completamente en las tres variantes.

La variante CRLF se construye a partir de los bytes de la fixture, no cambiando los offsets esperados después de normalizar la entrada. El comentario Unicode no convierte columnas en offsets UTF-8.


### 14.9. Proyecciones destruidas, prefijos y comparaciones excluidas

Se conserva la convención de marcadores y positivos pareados. Las etapas y campos restantes se obtienen de §§13.12–13.17.

| ID | Fuente negativa | Esperado | Positivo pareado |
|---|---|---|---|
| S29 | `fn main(){let a=[1];drop(⟦a[0]⟧);}` | NX-E0006, `feature_id = partial_transfer`, `stage = arguments`, `[25,29)` | Sustituir la proyección por `a`; destruye el binding completo. |
| S30 | `fn f(inout x:i64){} fn main(){f(inout ⟦1⟧);}` | NX-E0002, `stage = parse`, `[38,39)`, `parser_context = inout_place` | Insertar `var x=1;` antes de la llamada y sustituir el literal marcado por `x`. |
| S31 | `fn main(){let x=consume ⟦1⟧;}` | NX-E0002, `stage = parse`, `[24,25)`, `parser_context = consume_place` | Quitar `consume`; el literal ya produce un temporal propietario. |
| S32 | `fn main(){let x=1⟦>⟧2;}` | NX-E0002, `stage = parse`, `[17,18)`, `unsupported_binary_operator` | Sustituir `>` por `<`. |
| S33 | `fn main(){let x=1⟦<=⟧2;}` | NX-E0002, `stage = parse`, `[17,19)`, `unsupported_binary_operator` | Sustituir `<=` por `<`. |
| S34 | `fn main(){let x=1⟦>=⟧2;}` | NX-E0002, `stage = parse`, `[17,19)`, `unsupported_binary_operator` | Sustituir `>=` por `<`. |

S29 relaciona `drop`, `[20,24)`, con `role = context`. No se diagnostica copia de `a[0]` ni se ejecuta su destrucción.

Positivos adicionales:

```nexo
fn main(){drop([1][0]);}
```

Destruye un temporal y termina con unidad.

```nexo
fn drop(x:i64){}
fn main(){let a=[1];drop(a[0]);assert_eq(a[0],1);}
```

La función de usuario recibe lectura ordinaria; el array sigue disponible.

```nexo
fn main(){let r:Result<i64,i64>=Ok(1);assert_eq(r,r);}
```

Comprueba que reconocer `>=` como operador excluido no rompe el cierre de tipo seguido de asignación.

Para S32–S34 se repite además la variante con LF inmediatamente antes del operador. Debe conservar NX-E0002 sobre ese operador, recalculando los offsets sobre la fuente materializada; no se permite convertirlo en NX-E0005 con una reparación de inserción.

Las nuevas filas no añaden códigos ni `feature_id`, ni cambian los totales de trabajo ya publicados.
### 14.10. Inferencia diferida y fondo del retículo

Estas fuentes son ASCII sin LF final; offsets y grafías están sujetos a F0-D15. `DV` es únicamente una abreviatura documental de `(if true{return;}else{return;})`, que el generador sustituye literalmente; no es un nombre del programa. Las expectativas se transcriben de §4.1.1, no del solucionador probado.

| ID | Fuente completa o expansión exacta | Esperado literal |
|---|---|---|
| TD01 | `fn main(){var v=DV;var w=v;v=3;w=true;}` | Una NX-E0102/type_mismatch, `stage=types`, `operation=types.check`, primario `[62,66)` (`true` final), `data={"expected_type":"i64","found_type":"bool"}`. Relaciones `expected_type` sobre `3` en `[58,59)` y `found_type` sobre `[62,66)`. |
| TD02 | `fn main(){var v=3;var w=v;v=3;w=true;}` | Mismo código, subcausa, etapa, operación y `data` de TD01; primario y `found_type` `[32,36)`, `expected_type` `[16,17)` (el `3` que establece el tipo de `v`). |
| TD03 | `fn main(){var v=DV;var w=true;w=v;v=3;}` | Una NX-E0102/type_mismatch, `types`, `types.check`, primario y `found_type` `[65,66)` (`3`), `expected_type` `[54,58)` (`true` de `w`), `data={"expected_type":"bool","found_type":"i64"}`. La igualdad de `w` con `v` no se pierde. |
| TD04 | `fn main(){let x=(DV)==true;}` y `fn main(){let x=true==DV;}` | Ambos aceptados, `diagnostics=[]`; al ejecutar retornan unidad sin observaciones. La igualdad conserva resultado `bool` en comprobación hipotética; no emite `Never`. |
| TD05 | `fn main(){let x=[DV,true];}` y `fn main(){let x=[true,DV];}` | Ambos aceptados, `diagnostics=[]`; el elemento concreto impone `bool`, no un `i64` provisional. Ejecución unidad sin observaciones. |
| TD06 | `fn main(){var v=DV;var w=v;v=[1];w=[2];w[0]=9;}` | Aceptado; `v` y `w` comparten restricción `Array<i64>`, no datos. Gemelo vivo reemplazando DV por `[0]` y añadiendo `assert_eq(v,[1]);assert_eq(w,[9]);` antes del cierre: ejecución unidad, sin observaciones. |
| TD07 | `fn main(){let x=missing;let y=x;let z=true+1;}` | NX-E0101 por `missing` en la barrera `names`; ningún error derivado por `y` ni tipo inventado para `x`. Con `missing` sustituido por `0`, NX-E0102/type_mismatch sobre `true`, `expected_type=i64`, `found_type=bool`; alcanza `types`. |

Los pares de fondo tienen también negativos aislados: `fn main(){let x=DV+true;}` y `fn main(){let x=true+DV;}` exigen una NX-E0102/type_mismatch sobre el `true` no perteneciente a DV, `stage=types`, `operation=types.check`, `data={"expected_type":"i64","found_type":"bool"}`, `found_type` relacionado sobre ese mismo operando, sin `expected_type` fuente por ser dominio intrínseco. Sustituir ese `true` por `1` da el positivo. `[DV,1,true]` y `[1,DV,true]` dentro de un binding sin anotación exigen el mismo desajuste sobre el último `true`, relacionado con el primer `1` concreto; `Never` no oculta la contradicción.

La contaminación se aísla dentro de una sola etapa con `fn main(){let x=true+1;let y=x;let z=false+1;}`: exactamente dos NX-E0102/type_mismatch, sobre `true` y `false` en orden fuente, `expected_type=i64`, `found_type=bool`, sin diagnóstico sobre `y`. Sus positivos sustituyen ambos booleanos por `0`; corregir solo uno conserva únicamente el otro error. Estas variantes pertenecen al cierre de TD04/TD05/TD07, con campos restantes de §13.6, y no permiten que un tipo provisional sea aceptado como solución.

**Divergencias de implementación, dueño I07c.** TD01 y TD03 requieren la unificación diferida y la conservación de procedencias de §4.1.1. TD02 también diverge: el código actual relaciona `expected_type=[24,25)` (el nombre `v` de la asignación posterior), mientras que el esperado normativo es `[16,17)`, el literal `3` que fijó el tipo, conforme a §13.15. No se cambia ese esperado para coincidir con el código. Las tres obligaciones se cierran en I07c (§17.2).

Los positivos pareados TD01/TD02 sustituyen solo el último `true` por `4`; TD03 sustituye el último `3` por `false`. Deben aceptarse, también con restricciones procesadas internamente en orden inverso, con otra elección de representante y con recompilación debug/release. TD01–TD03 rechazados no ejecutan primitivas; `outcome.kind=rejected`, salida 1. Sus cadenas son `[]`, reparación `[]`, campos sintácticos nulos/listas vacías y `feature_id=null`; `rule`/`explanation_id=NX-E0102/type_mismatch`. No se suprime una contradicción porque sea hipotética. TD06 vivo verifica además independencia de copias (§6.1).

### 14.11. Ubicaciones y prioridad causal

Fuentes ASCII sin LF final, literales sujetos a F0-D15. Los tres fallos de índice siguientes tienen `code=R0102`, `subcause=null`, namespace `F0-0002-sim`, `stage=runtime`, `operation=value.index`, `data={}`, `cause_chain=[]`, `repair=[]`; resultado `simulator_fault`, salida 1, sin observaciones. La única relación es `context` en el rango indicado. El positivo sustituye el índice marcado `2` por `0` y termina normalmente.

| ID | Fuente | Primario | `context` |
|---|---|---|---|
| DG01 | `fn main(){let a=[[0]];print(a[0][2]);}` | `[32,35)`, segundo selector `[2]` | `[28,35)`, acceso completo. |
| DG02 | `fn main(){var a=[[0]];a[0][2]=5;}` | `[26,29)`, segundo selector `[2]` | `[22,29)`, destino completo. |
| DG03 | `fn f(inout x:i64){}fn main(){var a=[[0]];f(inout a[0][2]);}` | `[53,56)`, segundo selector `[2]` | `[43,56)`, argumento completo. |

Se repiten DG01–DG03 con el primer índice `2` y el segundo `0`: falla el primer selector, nunca el externo por defecto. DG01 positivo imprime `0`; DG02/DG03 positivos no emiten salida. Esas observaciones positivas no se atribuyen a los negativos.

| ID | Fuente | Esperado y positivo |
|---|---|---|
| DG04 | `fn main(){let a=[0];drop(a);a[0]=5;}` | Una NX-E0202/definitely_consumed sobre la raíz del destino `[28,29)`, `ownership.use`, `data={}`; NX-E0105 sería derivado del acceso indexado indisponible. Positivo: cambiar `let` a `var` y quitar `drop(a);`. |
| DG05 | `fn main(){let a=[0];drop(a);a=[1];}` | Una NX-E0105 sobre `[28,29)`, `place.write`, `data={}`: la sustitución completa no exige leer el valor destruido, pero infringe mutabilidad. Positivo: cambiar `let` a `var`. |
| DG06 | `fn main(){let a=[0];let b=0;drop(a);print(a);b=1;}` | Dos errores en `ownership`: NX-E0202/definitely_consumed sobre el `a` de `print` `[42,43)`, después NX-E0105 sobre el destino `b` `[45,46)`; `data={}` en ambos. Positivo: mover `print(a);` antes de `drop(a);` y declarar `var b`. |
| DG07 | `fn main(){var a:Array<i64>=[0];a[0]=true;}` | NX-E0102/type_mismatch sobre `true` `[36,40)`, `data={"expected_type":"i64","found_type":"bool"}`; `found_type=[36,40)` y `expected_type=[16,26)` sobre la anotación `Array<i64>` escrita, que impone el elemento. Positivo: `true` por `1`. |
| DG08 | `fn main(){var a=[0];a[0]=true;}` | Mismo código y `data`; primario y `found_type=[25,29)` sobre `true`; `expected_type=[16,19)` sobre el inicializador `[0]`, sin anotación inventada. Positivo: `true` por `1`. |

DG06: NX-E0202 relaciona `declaration=[14,15)` y `consumption=[28,35)`; cadena única `kind=drop`, `operation=value.drop`, span `[28,35)`, Entity `{"kind":"binding","id":"binding:14"}`, `evidence=verified`. Su NX-E0105 tiene `subcause=null`, `operation=place.write`, `declaration=[24,25)`, cadena `[]`. DG07/DG08 tienen `stage=types`, `operation=types.check`, cadena y repair `[]`; las únicas relaciones son expected_type y found_type, ordenadas por span. DG04, DG07 y DG08 **divergen hoy del código**: DG04 prioriza indebidamente NX-E0105; DG07/DG08 relacionan el destino `a[0]` en `[31,35)`/`[20,24)` en lugar del origen de restricción exigido. Son obligaciones, no esperados alternativos.

DG04/DG06 conservan declaración, consumo y testigo de §13.15; DG05/DG06 relacionan la declaración inmutable. La disponibilidad no prevalece por el número del código: prevalece cuando causa el otro defecto. Los spans de R0001/R0002 por unidad final y sus ticks quedan fijados en §§11.6–11.7; el agotamiento por construcción de esa unidad relaciona el cuerpo y señala únicamente su cierre.

## 15. Arnés diferencial y adversarial

Se requieren arneses nuevos. Los existentes sirven como antecedente y fuente de regresiones, no como evidencia suficiente del corte ampliado. Se distinguen pruebas independientes desde fuente y pruebas internas de representación.

### 15.1. Modelo diferencial independiente

Un modelo Python de biblioteca estándar ejecuta una representación de pruebas propia con:

- Entorno de bindings y propietarios.
- Préstamos y représtamos lógicos.
- Frames, ramas y bucles.
- `Ok`/`Err` y propagación.
- Enteros comprobados y comparaciones de §4.6.
- Colas mock del esquema de §12.4.
- Registro ordenado de observaciones y reservas de cierre.
- La contabilidad publicada del perfil.

El generador produce simultáneamente fuente Nexo, representación del modelo y JSON del entorno. No obtiene esperados ejecutando el compilador Nexo ni leyendo su IR.

Invoca la CLI con `--format json`, `--mock-env` cuando corresponda y el perfil literal. El modelo calcula las líneas virtuales, incluidos prefijos y LF, y no mide la longitud del JSON devuelto por el compilador.

Semilla inicial registrada: `20261020`. Debe cubrir:

- Llamadas adelantadas, anidación y recursión limitada.
- Permutaciones de argumentos nombrados.
- Copias y modificaciones de arrays anidados.
- Propiedad distinta por rama.
- Reinicialización antes y después de retroaristas.
- Errores en cada posición de argumento, operando, retorno y `?`.
- Recursos creados, transferidos, encapsulados y reemplazados.
- Colas mock vacías, agotadas y de longitud variable.
- Comparaciones de arrays y `Result`, incluidas diferencias de longitud y variante.
- Reservas de cierre que compiten con salida y eventos.

Se comparan exactamente resultado, código de salida del proceso, salida de programa, eventos, registro intercalado, bytes emitidos, reservas finales y primer fallo entre modelo, debug y release. Los spans se comparan exactamente salvo `primary_span` de NX-E0003 para `type_solver_steps`, `effect_fixpoint_steps`, `ir_items` y `ownership_flow_steps`: se comparan `code`, `stage`, `data.budget_kind`, `data.limit`, `data.observed`, `data.budget_scope`, `data.incomplete_analysis` y, cuando corresponda, `data.analysis_function`, y se verifica por propiedad que cada span exista, identifique el archivo de entrada y cumpla `0 <= start < end <= longitud_utf8`, con extremos en fronteras de caracteres. No se compara ese span con un offset esperado del modelo ni entre builds. Los otros seis `budget_kind` conservan spans exactos. Solo se normaliza la ruta temporal asignada a una fixture; no se normalizan offsets, CRLF ni contenido.

Los casos de cuotas utilizan §§11.3–11.8, incluidos los perfiles normales y reducidos de análisis. Un fallo por una cuota distinta de la objetivo es discrepancia, aunque la ejecución termine de forma controlada.

### 15.2. Mutaciones adversas desde fuente y entrada

El arnés aplica:

- Truncamiento en cada frontera válida de UTF-8.
- Bytes UTF-8 inválidos como pruebas de entrada separadas.
- Inserción/eliminación de delimitadores y saltos.
- Sustitución de `consume`, `inout`, etiquetas y terminadores.
- Cadenas y árboles compuestos profundos.
- Tipos profundos construidos a través de bindings y llamadas.
- SCC grandes, bucles con consumos y cadenas de efectos.
- Sombreado de funciones, `value` y nombres de intrínsecos.
- Entradas de límite `N-1`, `N`, `N+1`.
- JSON inválido, claves duplicadas, campos desconocidos, valores `i64` no canónicos, archivo excesivo y colas agotadas.

Cada negativo semántico debe llegar a su etapa y causa. Una mutación que produce error sintáctico solo cuenta como robustez del frontend. Un error de configuración del entorno solo cuenta como prueba del controlador.

Las guardas de análisis se aíslan con los perfiles y generadores de §§11.4 y 11.8. El informe separa frontera del contador, alcanzabilidad desde fuente y diferencial de ejecución; no presenta una prueba unitaria del contador como validación independiente del análisis completo.

### 15.3. Propiedades metamórficas desde Python

- Renombrar bindings sin captura accidental conserva resultado y efectos.
- Permutar declaraciones de funciones conserva resolución.
- Añadir paréntesis que preservan `Place` conserva permisos.
- Sustituir un terminador insertado por `;` conserva comportamiento y diagnóstico pertinente.
- Introducir un wrapper directo conserva efectos transitivos.
- Cambiar la implementación de una hoja obliga a recalcular su SCC y llamadores en una compilación nueva.
- Cambiar debug/release no altera resultados, fallos, eventos ni bytes virtuales.
- Cambiar `--format text` por `--format json` conserva ejecución y cuotas; el arnés compara las proyecciones publicadas, no el tamaño de ambos transportes.

No se afirma invalidación incremental: no hay caché persistente en este corte.

### 15.4. Impresor y verificador: pruebas internas separadas

`tests/canonical_print.rs` usa el impresor de §10. Parsear su salida conserva el AST normalizado y volver a imprimir produce los mismos bytes. Los programas formateados también pueden ejecutarse para contrastar observaciones, pero esa generación mediante el impresor Rust no se presenta como independencia del frontend.

`tests/ir_verifier.rs` parte de IR válida de prueba y elimina por separado metadatos de efectos, préstamos o limpieza; también altera tipos, destinos, flags y procedencias. Cada mutación pertinente debe ser rechazada antes de obtener un artefacto ejecutable.

Estas mutaciones de IR son **pruebas Rust del verificador**, no propiedades ejecutadas por el arnés Python ni negativos fuente con códigos NX-E inventados. Se registra para cada una la invariante vulnerada, la mutación y el rechazo. La API pública continúa sin aceptar IR no verificada.
## 16. NB1–NB8 y trazabilidad

| Pendiente | Decisión |
|---|---|
| NB1, `drop(p)` | Resuelto para el corte en §6.5: intrínseco experimental, consumidor y con grafía provisional. |
| NB2, flotantes frente a `..` | Resuelta la frontera léxica en §3.2. Semántica flotante y rangos ejecutables diferidos. |
| NB3, `BaseAttribute` sin producción | Diferido. La gramática aceptada de este corte no referencia `BaseAttribute`; `@` recibe diagnóstico de alcance. Antes de implementar atributos deberá cerrarse su producción y corpus. |
| NB4, patrón tupla sin tipo/expresión | Diferido conjuntamente: no patrones, tipos ni expresiones tupla. `()` es unidad y `(x)` agrupación; una coma de producto se reconoce para el diagnóstico de alcance. |
| NB5, explicación consultable | Resuelto en el esquema propuesto y `nexo explain`, sin afirmar LSP/REPL. |
| NB6, activación de `extend` local | Diferido explícitamente con módulos/extensiones. Este corte no establece activación implícita ni cambia D08/D13. El siguiente ADR de módulos deberá decidir el caso local y aportar el par con/sin `use extension`. |
| NB7, negativos sin positivo | Se exige par completo para cada regla implementada y cada subcausa emitida. Las filas antiguas fuera de alcance conservan su pendiente. |
| NB8, vacío `value:` frente a E0005 | Resuelto en §13.1 y B05/B08, con prioridad local y sin aceptación por recuperación. |

Cobertura prevista, siempre parcial:

- T01/T02: datos independientes, recursos y transferencia.
- T07/T08: evaluación, límites enteros, `Result`, salidas y limpieza.
- T10: no retención de préstamos durante llamadas; no vistas generales.
- T14: efectos transitivos, limpieza y desconocidos.
- T21: etiquetas y orden de argumentos; no texto/defaults.
- T25: códigos, spans, cadenas, explicaciones y reparaciones.

T03–T06, T09 completo, T11–T13, T15–T20 completos, T22–T24 completos y T26–T28 mantienen sus obligaciones. Tampoco se acreditan H-PY/H-RS/H-CS, los modelos D09, ABI/backend ni cuatro plataformas.

## 17. Orden de implementación

Cada subcorte entrega una capacidad y sus comprobaciones locales. Una prueba que necesita una capacidad posterior no cierra un paso anterior. Los subcortes preparatorios son pruebas internas del desarrollo; no anuncian conformidad completa con `f0-0002-v2` ni un PASS del corte.

| Paso | Trabajo | Comprobación de salida |
|---|---|---|
| I01 | Esquema diagnóstico, spans, presentación, catálogo de `explain` y límites de entrada. | Serializar objetos construidos de prueba, incluidos NX-E y R; validar campos y vocabularios; texto, Unicode, lectura acotada y errores de uso. No exige que un frontend o runtime todavía inexistente produzca esos objetos. |
| I02 | I02a: léxico, tokens y spans. I02b: parser, terminadores, varias firmas, tipos escritos, formas reconocedoras e impresor. | I02a cierra negativos LX01–LX05. I02b cierra AST normalizado, spans, round-trip, operadores excluidos, prefijos y `value` contextual; L02 negativo completo en `parse`. Para las demás L, únicamente las aserciones de parseo indicadas abajo. No se exige `check` semántico ni `run`. |
| I03 | Nombres, asociación y tipos básicos; retornos escritos y contextos de tipo de expresiones sin control ni Result. | Duplicados, destinos, etiquetas, modos estructurales, sombreado, literales y diagnósticos de tipos independientes. Cierra los negativos L01 y L07 en sus etapas. No ejecuta llamadas ni exige préstamos, efectos o análisis de caminos. |
| I04 | I04a: IR y verificador inicial, intérprete escalar, flujo lineal, bindings, asignación completa, primitivas de datos y registro de salida en `interp/observation.rs`. I04b: arrays, copias, índices y asignación indexada sobre raíces propietarias locales. | I04a cierra `integer_limits`, literal fuera de rango, basura final, RT01/R0103 y L04 por resultado y AST. I04b cierra `immutable_write.nx`, `nested_copy`, arrays heterogéneos, positivo L07 e índices/RHS. Solo se ejecuta la IR admitida por cada subpaso, con datos sin recursos ni llamadas de usuario; la contabilidad completa espera I10. |
| I05 | Frames y llamadas Nexo, parámetros ordinarios de lectura sin accesos exclusivos, recursos, `consume`, retorno explícito y limpieza secuencial; eventos de recursos en `interp/observation.rs`. | Transferencia, sombreado, retorno, copia prohibida y usos posteriores a consumo/drop; fallos entre argumentos, orden escrito y errores secuenciales después de return. L03 completo; positivo L01 completo. `drop` completo o temporal y rechazo de destrucción parcial. Eventos de adquisición, inspección y cierre ordenados; sus cuotas y reservas se cierran en I10b. |
| I06 | Préstamos explícitamente representados, `inout`, conflictos y représtamos. Extensión del Place ya existente a raíces prestadas y modificaciones mediante llamadas. | Exclusividad, permisos, copia independiente devuelta y ausencia de escape del subconjunto. Índices calculados antes del RHS y comprobados después cuando el RHS llama o modifica legítimamente la raíz. |
| I07a | Ramas, valores de rama, divergencia y construcción de CFG de `if`/`value:` y `while`. | Comprobación y ejecución correctas en su alcance: uniones acíclicas, continuaciones sin valor, retorno faltante, limpieza y código hipotético. No cierra propiedad dependiente de retroaristas ni inferencia diferida monomórfica. |
| I07b | Propiedad entre iteraciones de bucles: condición, cuerpo y retroaristas. | P09; consumo posible/definitivo, reinicialización, testigos, conservación de diagnósticos y terminación acotada (§6.3). Incluye flujo hipotético de bucles. |
| I07c | Inferencia coherente de tipos diferidos (§4.1.1) y conservación de orígenes de tipo esperado DG07/DG08. | Variables monomórficas relacionadas, Never separado de incógnita/contaminación, orden del solucionador irrelevante; TD01–TD07. Cierra ambas lagunas de tipado antes de I08. |
| I08 | `Result`, constructores, `?`, `try_add` y comprobación completa de entrada de `run` (NX-E0112). | Resultados y limpieza de payloads, copia prohibida del tipo completo, propagación entre frames y separación de Err frente a R; `missing_main`, `invalid_main_signature` y sus positivos de §14.1, incluidas las firmas Result admitidas. Los ejemplos de §11.7 se comprueban por resultado; sus totales todavía no cierran este paso. |
| I09 | Manifiesto y análisis transitivo de efectos antes y después del lowering; metadatos y verificador de efectos. | Destructores, desconocidos, SCC, ramas no ejecutadas, helpers, cadenas canónicas y refinamientos de variante justificados. Los casos de este paso son finitos y no requieren todavía demostrar fronteras de cuotas. |
| I10 | I10a: cuotas completas de análisis, unidades dispersas, límite local/global, continuación y profundidad compartida por parser/lowering. I10b: trabajo, cuotas estructurales de ejecución, bytes de observación y reservas de cierre en `interp/observation.rs`. | I10a cierra perfiles reducidos local/global, B-A1–B-A4, B-L1/B-G1/B-D1, frontera 127/128 y contrato de spans de §13.13: seis presupuestos con igualdad exacta y cuatro con propiedad de validez y no vaciedad. I10b cierra ticks 2/6/1 de §11.7, spans DG01–DG03 y unidad final, totales 56/25/15, familia `999992+p`, celdas, frames, observaciones y reservas; también la igualdad de trabajo entre variantes de L04. |
| I11 | I11a: interfaz mock completa y tres flujos. I11b: suite integrada y arneses independientes. | I11a cierra colas, JSON y eventos de las cuatro primitivas nuevas, usando el registro y las cuotas ya disponibles. I11b contrasta debug/release, suite migrada íntegra, reparaciones y matriz V01–V13; distingue las mutaciones internas de IR del arnés Python. Repite los cierres locales sin posponer hasta aquí su primera comprobación. |

### 17.1. Matriz de dependencias comprobada

La columna central enumera capacidades necesarias, no solo números de pasos. Cada fila utiliza capacidades ya disponibles o introducidas por ella misma. I02, I04, I10 e I11 son grupos ordenados de dos subpasos: `a` solo requiere los grupos anteriores de la matriz; `b` requiere además el `a` de su grupo. Toda referencia posterior a esos grupos exige ambos subpasos. I07 tiene tres subcortes ordenados: I07a, I07b e I07c; «I07 terminado» exige los tres cerrados. I08 depende expresamente de I07c. I10a no requiere las cuotas de ejecución de I10b; I11a no requiere el arnés independiente de I11b. Así, la partición conserva el orden y no introduce dependencias hacia delante ni ciclos.

| Paso | Capacidades anteriores requeridas | Dependencias anteriores |
|---|---|---|
| I01 | Ninguna capacidad del compilador ampliado. | — |
| I02 | Fuente acotada y diagnóstico serializable. | I01 |
| I03 | AST, tokens, spans y reconocedores. | I01–I02 |
| I04 | Nombres y tipos básicos; AST de asignaciones e índices. | I01–I03 |
| I05 | Valores, IR, primitivas, slots y flujo lineal; firmas y asociación. | I03–I04 |
| I06 | Place local de I04; frames, propietarios y llamadas de I05. | I03–I05 |
| I07a | Tipos básicos, IR, llamadas, propietarios y permisos. | I02–I06 |
| I07b | CFG y divergencia; propietarios y permisos. | I06, I07a |
| I07c | Comprobación viva/hipotética y flujo de ramas/bucles. | I03, I07a–I07b |
| I08 | Llamadas, limpieza, permisos, control por caminos e inferencia diferida coherente. | I03, I05–I06, I07a–I07c |
| I09 | Operaciones, llamadas, control, Result y planes de limpieza. | I03–I08 |
| I10 | Todos los recorridos y operaciones cuyo coste se mide. | I01–I09 |
| I11 | Implementación de todas las capacidades y cuotas del corte. | I01–I10 |

No hay dependencia hacia un paso de número mayor. En particular:

- La asignación por lugar lógico nace en I04 para raíces locales propietarias. I06 amplía sus permisos y sus interacciones con llamadas; no vuelve a introducir una capacidad necesaria para `nested_copy`.
- I03 comprueba el tipo de un retorno escrito, pero no acredita ausencia de caídas por caminos. I07a cierra NX-E1317 para sus filas; las variantes cuya disponibilidad depende de retroaristas requieren I07b.
- I08 comprueba semántica de Result; I10 comprueba sus ticks.
- I01 prueba el serializador y el catálogo con objetos y fuentes documentales, sin requerir emisores futuros ni ejecutar los ejemplos de `explain`.
- Las garantías completas del verificador —incluidos préstamos, control y efectos— se incorporan con I06, I07 e I09. El artefacto público del perfil completo solo se acredita al completar I11.

### 17.2. Cierre preciso de las fixtures L

| Fixture | Aserción permitida en I02 | Primer cierre posterior de la obligación restante |
|---|---|---|
| L01 | AST con retorno vacío seguido de expresión descartada; spans y AST del positivo con agrupación abierta antes del LF. | Negativo NX-E0102 en I03; positivo completo con retorno explícito en I05. |
| L02 | NX-E1302/missing_value_operand, campos y spans exactos; el positivo supera parseo. | Positivo completo del `if` expresión en I07a. |
| L03 | Una llamada cuyo nombre `value` y `(` están en líneas distintas. | Comprobación y ejecución en I05. |
| L04 | Mismo AST normalizado para ambas grafías y terminadores insertados. | Comprobación y resultado en I04; igualdad de trabajo en I10. |
| L05 | ValueStmt con operando unidad y terminación correcta. | Comprobación y ejecución en I07a. |
| L06 | ExprStmt terminada por LF; ausencia de ReturnStmt. | NX-E1317 y positivo completo en I07a. |
| L07 | Tokens, comentario Unicode y spans de las tres materializaciones. | Negativo NX-E0101 en I03; positivo completo de bindings en I04. |

Una aserción parcial de parser no acredita el positivo pareado completo exigido en §14. El cierre íntegro se registra en el paso indicado, y la suite completa vuelve a contrastarse en I11.

El resto de filas se cierra localmente según esta distribución. Cerrar significa comprobar el negativo y su positivo completo, y ejecutar las observaciones que la fila exija; no incluye todavía cuotas de I10 ni el contraste independiente de I11b. Los negativos que solo requieren parseo o nombres pueden probarse antes, sin acreditar entonces la pareja completa.

| Paso de cierre | Filas completas |
|---|---|
| I04, en `a` o `b` según requieran arrays/índices | S10, S16–S18, S22–S23, S25, S27–S28, S31–S34; F03–F06, F08, F16–F19, F21–F24, F27–F28, F32–F33; LX01–LX05 y RT01. |
| I05 | S01–S05, S07, S12–S15, S19, S24, S26, S29; P01–P04; PP01, PP03–PP04; B07; F01–F02, F07, F09–F12, F20, F29–F31. |
| I06 | S06, S08, S30; P05–P07 y su `data` exacta; PP02. |
| I07a | S11; P08; B01–B06, B08–B09; F14, F25–F26; positivo L02 y L05–L06. |
| I07b | P09 y todas las variantes de propiedad cuya causa depende de condición, cuerpo o retroarista de bucle; incluye usos posteriores, reinicialización, código hipotético y conservación de diagnósticos convergidos. |
| I07c | TD01–TD07; variantes diferidas de ramas, arrays, igualdad y asignaciones de §4.1.1. Incluye las divergencias TD01/TD03 y el origen de tipo de TD02: expected_type [16,17), frente a [24,25) del código actual (§14.10). |
| I08 | S09, S20–S21; P10–P11; PP05; R01–R03; F15; negativos y positivos de entrada NX-E0112. |
| I09 | E01–E06, W01–W07, F13 y PP06. |
| I10a | B-A1–B-A4, B-L1/B-G1/B-D1 de §11.8; cuotas reducidas local/global y profundidad 127/128. B-G1 usa 14 funciones N más main: 168.813 bytes, 162.309 ítems y 4.282.686 unidades completas medidas; el arnés baja toda la IR antes de propiedad e informa ir_items acumulado (§11.8.B). |
| I10b | DG01–DG03 y spans/ticks de unidad final; no es propietario de correcciones de prioridad o procedencia de tipos. |
| I07b, corrección de propiedad | DG04: prioridad causal disponibilidad/mutabilidad; DG05: sustitución completa y mutabilidad; DG06: conservación de dos errores independientes y relaciones. Las tres requieren la misma frontera de diagnósticos de propiedad, ya disponible; deben cerrarse antes de I07c. |
| I07c | DG07/DG08: origen real de expected_type anotado/inferido; comparten representación de restricciones y procedencias con TD01–TD07, antes de I08. |

Esta tabla cubre S01–S34, P01–P11, PP01–PP06, R01–R03, B01–B09, E01–E06, W01–W07 y F01–F33 sin exigir implementar las características rechazadas por NX-E0006. Las nuevas TD01–TD07 se cierran en I07c; DG04–DG06 tienen propietario I07b y DG07/DG08 I07c, con las dependencias y razones de la tabla; I10b solo repite estas regresiones. La regla de spans de presupuestos se cierra en I10 y se contrasta de forma independiente en I11b.

Las pruebas adversariales que requieren cuotas completas se ejecutan después de I10; los pasos anteriores usan fixtures finitas y acotadas. Las partes preliminares no se publican como garantías verificadas del perfil completo.

Cada regla exige negativos aislados con código, subcausa, span, token y campos exactos; esperados transcritos de este ADR; positivos contra falsos rechazos; mutación inversa por regla y restauración SHA-256; fuzz sin pánicos y nombres de test sin recuentos de pasadas. Ninguna diferencia documentada se cierra ajustando el esperado al código.

Al cerrar cada subcorte implementado, el responsable actualiza `Pending.md`, `HANDOFF.md` y `RECOVERY.md`, registra defectos y regresiones y sigue el flujo de commit, push sin force y comprobación de HEAD remoto de `AGENTS.md`. Esta entrega documental no realiza esas operaciones.
## 18. Criterios exactos para Flujo B

El validador será distinto del implementador y recibirá código, este ADR, catálogo, fuentes normativas y criterios; no una conclusión previa que deba ratificar.

El informe utiliza por requisito **PASA / FALLA / NO EVALUADO / INCONCLUSO**, con archivo/línea o comando, entrada, salida y revisión del código.

| Requisito | Condición necesaria |
|---|---|
| V01, alcance | Todas las construcciones aceptadas están declaradas; las diferidas se identifican sin ejecución parcial. |
| V02, frontend | Terminadores y `value:` se deciden sin tipos; negativos alcanzan token/span previstos. |
| V03, llamadas | Orden escrito, asociación correcta, recursión y firmas sin inferencia global accidental. |
| V04, propiedad | Ninguna copia de recurso, uso inválido, doble propietario o limpieza doble; ramas/bucles conservan estados y testigos. I07 requiere I07a, I07b e I07c; TD01–TD07 prueban restricciones monomórficas sin compartir datos. |
| V05, préstamos | Conflictos por origen, représtamos subordinados y ausencia de escape en todo valor aceptado. |
| V06, `Result` | `?` extrae o retorna una sola vez, conserva error tipado y limpia; no captura R ni revierte efectos. |
| V07, IR | Ejecución exclusiva de IR verificada; tipos, procedencia, efectos y limpieza inspeccionables. |
| V08, efectos | Desconocido no vacío; cadenas completas; limpieza incluida; recomprobación tras lowering. |
| V09, robustez | Cuotas locales/globales aisladas y deterministas; aceptación B-A y rechazo B-L/B-G/B-D; subconjunto exacto de diagnósticos convergidos, continuación posterior dentro del saldo global y ningún artefacto verificado con análisis incompleto. Profundidad 127/128 coherente, ticks 2/6/1 normativos y limpieza tras agotamiento. |
| V10, diagnósticos | Código/subcausa, ubicación, explicación y reparación correctos; aplicabilidad solo tras recomprobar. V09/V10 aplican §13.13: para los cuatro presupuestos costosos, span válido y no vacío, sin igualdad de offsets. |
| V11, flujos | Los tres programas reducidos ejecutan lógica general con entradas variables y fallos; no respuestas elegidas por fixture. |
| V12, independencia | Modelo diferencial y adversarial separados, resultados íntegros y discrepancias reducidas a regresiones. |
| V13, conservación | Base intacta, migración explícita y pendientes visibles; no reinterpretación de pruebas históricas. |

Se ejecutan formato, clippy estricto y tests offline/locked en debug y release, además de los arneses independientes actualizados. Los binarios se reconstruyen para la revisión evaluada.

Para conceder PASS del corte, V01–V13 deben tener evidencia suficiente y no quedar FALLA, NO EVALUADO o INCONCLUSO en obligaciones incluidas. Un defecto corregido exige regresión y reejecución afectada. El recuento de tests y las demos no sustituyen esos criterios.

Un PASS del corte no aprueba F0, F1 ni familias T completas. Permanecen la puerta de dos revisores sin discrepancias, la aprobación del propietario y todas las obligaciones diferidas.

## 19. Alternativas descartadas y riesgos

Se descarta conservar el AST como única representación ejecutable: no permite comprobar independientemente el lowering requerido por D04.

Se descarta inferir transferencia cuando una copia no es admisible: cambiaría silenciosamente propiedad al editar un tipo.

Se descarta tratar funciones sin información de efectos como puras o fiarse de que una ejecución mock no mostró un efecto.

Se descarta adelantar closures y generadores sin definir retención y estado. F0-0003 deberá preservar creación sin ejecución, `next` con efecto `suspend`, `gen fn -> T` como tipo de elemento, final normal o `return;`, rechazo de `return valor`, propiedad de capturas y limpieza una vez.

Los puntos de menor confianza son:

- La exclusividad por raíz completa puede rechazar usos útiles de elementos disjuntos.
- Los préstamos mantenidos durante toda la llamada pueden obligar a preparar temporales explícitos.
- La unión conservadora de bucles y ramas puede perder correlaciones válidas.
- Incluir limpieza por cuotas en los efectos hace restrictivas ciertas transferencias de recursos.
- Las cuotas lógicas no constituyen límites duros de memoria o tiempo del proceso.
- La separación entre efectos del modelo e infraestructura anfitriona necesita presentación clara.
- La contabilidad, los márgenes de alcanzabilidad y el esquema observable todavía requieren contraste con una implementación.
- Diferir closures, generadores, módulos y D09 deja puertas importantes de F0 abiertas.

Los pendientes históricos de D11–D13 son NB1–NB8 (§16). Las rondas síncronas de efectos, la contabilidad lógica pese al interning y la selección canónica de testigos requieren contraste con su implementación. Las mediciones explícitas de §11.8.B no acreditan la conformidad del perfil completo ni la aprobación de F0.

## 20. Enmiendas

Vigencia de las decisiones siguientes: **2026-10-22**, salvo fecha explícita. Sustituyen las reglas anteriores en sus secciones; no crean otro dialecto ni acreditan implementación o aprobación de fase.

| Decisión | Regla vigente y motivo |
|---|---|
| Partición de I07 | §§17–17.2 distinguen I07a (ramas/CFG), I07b (propiedad iterativa) e I07c (inferencia diferida). I07 exige las tres; I08 depende de I07c, para no construir Result sobre tipos incoherentes. |
| Tipos diferidos y fondo | §§4.1.1, 6.2, 13.17 y 14.10 fijan unificación monomórfica, relación entre copias de bindings, independencia del solucionador y separación Never/incógnita/contaminación. Cierran `adr_gap_never_bottom` y `adr_gap_deferred_var_alias` sin cambiar la semántica de copia. |
| Cuotas dispersas | §§11.1, 11.4, 11.8 y 13.9 adoptan unidades dispersas versionadas en `f0-0002-v2`, prioridad por índice con punto fijo local en CFG estructurados, límite por función y global, diagnósticos convergidos y continuación acotada. Sustituyen cobros densos artificiales y garantías basadas solo en bytes; implementación global completa en I10a. |
| Índices y tipo esperado | §§13.12, 13.15 y 14.11 fijan selector fallido común a lectura/escritura/inout y relaciones con el contexto completo; el tipo esperado relaciona la anotación o restricción real, sin inventar fuente. Permite localizar y reparar la causa. |
| Prioridad causal | §§13.9 y 14.11 distinguen NX-E0105 de NX-E0202 por dependencia causal, conservando errores independientes; el número del código no decide prioridad. |
| Profundidad compartida | §§11.1 y 13.13 fijan la misma altura para parser y lowering y el caso 127/128 términos. Evita filtrar una guarda interna como exclusión o conclusión definitiva. |
| Trabajo y unidad final | §§11.6–11.7 conservan 2 ticks para función vacía y 6 para asignación indexada; el modelo cobra operaciones del lenguaje, no instrucciones administrativas. R0001/R0002 por unidad final señala la llave de cierre. Las diferencias se corrigen en I10b. |
| Catálogo integrado | §13 y F0-CODIGOS integran códigos, subcausas, perfiles, serialización y trazabilidad del addendum, que se elimina para mantener un solo registro. |
| Superficie única | La nota inicial remite la grafía oficial a F0-D15; los literales dependientes de grafía quedan sujetos a ese ADR. Se conserva aquí la semántica sin inventar una superficie nueva ni mantener compatibilidad. |
| Aislamiento de B-D1 | §11.8.B sustituye bucles planos por 96 bucles anidados y 6.000 bindings; 61.407 ítems (61.421 con later y su IR parcial) quedan bajo 262.144 y propiedad rechaza el cargo 1.000.001. Se conserva el testigo previo y la continuación a later. |
| Ubicación del límite local | §§11.1 y 11.8.B exigen primario dentro de analysis_function; B-D1 lo restringe además al sufijo anidado, posterior al uso conservado. La lista ordenada deja de depender de un span arbitrario. |
| Literales de procedencia y prioridad | §14.11 fija primarios, relaciones y cadena de DG06–DG08; declara las divergencias actuales DG04/DG07/DG08 para impedir su cierre por ajuste del esperado. |
| Funciones sin bucles | §11.1 asigna a I10a el cobro de flujo de funciones planas y el negativo flow-32, hoy omitidos por defer_ownership. |
| Orden exacto del punto fijo | §11.1 especifica cola de prioridad por índice, equivalente a RPO solo para la construcción de CFG estructurados descrita; no presupone esa equivalencia para cualquier grafo. |
| Diferencias completas de ticks | §11.7 registra además 67 frente a 56 en el bucle finito y 0 frente a 1 para el bloque independiente; I10b conserva el modelo normativo. |
| Propietarios de DG04–DG08 | §17.2 asigna DG04–DG06 a corrección de I07b y DG07/DG08 a I07c por sus dependencias de propiedad y procedencia de restricciones; I10b no absorbe esas correcciones. |
| Retirada física y CLI única | F0-D15 §§1 y 7 elimina en el corte B syntax.rs, runtime.rs y las API/partes heredadas; check/run usan el núcleo nuevo verificado. No se pospone ningún frontend a I11. |
| Corpus heredado literal | F0-D15 §7 reescribe manualmente los 15 archivos de F0-0001 y sus tests; la extracción AST se limita a fixtures que el frontend F0-0002 realmente admite. |
| Rechazo de capacidad pendiente | F0-D15 §§3.6, 6.1 y 7.1 concreta NX-E0006 con implementation_limit, sustituyendo legacy_move en el conjunto B de 30 entradas; pending_scope identifica I08/I09/I10a/I10b/I11a. No reintroduce el prototipo para cubrir capacidades pendientes. |
| Perfiles B derivados | F0-D15 §6.1 deriva el identificador del sufijo vigente de F0-0002, incluidos los ocho reducidos y flow-global-32; no congela una versión anterior. |
| Corpus B de la enmienda | F0-D15 §§8.12–8.13 transcribe T01–T07/D01–D08, variantes y B-D1 con sus bytes y relaciones nuevos, conservando las reglas semánticas de §§14.10–14.11. |
| Lookahead explícito | F0-D15 §3.1 fija LL(2) contextual en F0 y análisis especulativo sensible a espacios para GenericHead de B completa; no presenta ambos como LL(1). |
| Frontera léxica de cadenas | F0-D15 §§2.4 y 3.6 separa la envoltura/escapes/llaves de F0 del análisis recursivo de interpolaciones de B completa; text se emite solo para un token léxico válido. |
| Nombres retirados sin reserva | F0-D15 §2.2 deja inout/consume como Name sin modos ni prefijos; no reserva rastros de la gramática anterior. |
| Offsets independientes | F0-D15 §§6.1 y 7 fija contexto R0102 completo, offsets conservados por main→Main y escaneo de bytes independiente en tests; las filas de ADR se transcriben a mano, nunca del mapa del impresor como único oráculo. |
| Diagnóstico de lógica excluida | F0-D15 §§3.2 y 6.3 fija contextos, tokens, FIRST, prefijos y primarios literales para not/and/or en F0, sin ampliar operadores aceptados. |
| Terminación explícita canónica | F0-D15 §§2.5, 3.2 y 8.1 distingue entradas con salto de salida fmt con punto y coma; advierte continuidad ante paréntesis, corchete y menos y fija la selección descartada agrupada. |
| Sobrecargas mantenidas | F0-D15 §2.4 conserva las de D14 §6.2 como deuda aceptada por el coordinador frente a F0_ACUERDOS §9.6, criterio 3; se justifican por contextos y se miden sin atribuir cumplimiento perfecto. |
| Aislamiento y orden del arnés de propiedad (2026-10-23) | §11.8.B reduce B-G1 a 14 funciones N más main y exige bajar toda la IR construible antes de analizar propiedad, informando ir_items acumulado, incluidos registros parciales. Evita que otra cuota haga inalcanzable el rechazo global; §17.2 lo asigna a I10a. |
