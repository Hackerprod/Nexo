# F0-D14 — Sintaxis de superficie: alternativas, medición y migración

## 1. Estado, alcance y autoridad

Fecha: 2026-10-20. Responsable: Codex, diseñador del Flujo A. Estado: **propuesta para revisión independiente; sin mediciones ni decisión de congelación**. El punto de decisión es **antes de cerrar F1**. Este ADR no acredita ejecución de sus programas ni aprobación de ninguna fase. Su entrega se limita a este archivo.

Referencias locales, con abreviaturas usadas en las citas del resto del documento:

| Referencia | Archivo y secciones pertinentes |
|---|---|
| R | [Nexo_ROADMAP_rev0.2.md](../baseline/Nexo_ROADMAP_rev0.2.md), §0.2 y EX01–EX04; P01–P09; §2; D01, D02, D08, D10; F0–F6; §7. |
| AC | [F0_ACUERDOS.md](../F0_ACUERDOS.md), §§1–4, 6–9, especialmente §9.6, aclaración, cinco criterios y puntos a resolver. |
| A1 | [F0-0001-first-slice.md](F0-0001-first-slice.md), «Gramática del subconjunto», «Tipos y acceso», «Evaluación y limpieza». |
| A2 | [F0-0002-second-slice.md](F0-0002-second-slice.md), §§2–4, 6–10, 12, 13, 15 y 17. |
| C11 | [F0-D11-closures.md](F0-D11-closures.md), §§1–5.3, 6–10. |
| C12 | [F0-D12-async.md](F0-D12-async.md), §§1–10. |
| C13 | [F0-D13-expresiones-modulos.md](F0-D13-expresiones-modulos.md), §§1–12 y gramática G.1–G.3. |
| EXT | [sintaxis-candidata-externa-1.md](../proposals/sintaxis-candidata-externa-1.md), §§1–22. Es una candidata, no una decisión. |
| AG | [AGENTS.md](../../AGENTS.md), roles, Flujo A, reglas comunes, lecciones y autoridad delegada. |

La autoridad sigue siendo R y AC; no se modifica la base ni se crea una revisión 0.3 (AG, objetivo; AC §1). R §2 pide integrar características por su función y sus interacciones, no acumular procedencias. AC §9.6 añade expresamente PascalCase, equilibrio entre cohortes y medición de utilidad para agentes. Aquí «agente amigable» significa generar y reparar correctamente con información local, diagnósticos estables y un presupuesto acotado; es una hipótesis medible, no una cualidad demostrada por usar ciertas palabras.

Distinción necesaria: A1 exige `;` y usa `move`; A2 §3 introduce terminadores de C13 y `consume`. A2 §2.2 difiere cadenas, módulos, closures, rangos y async, entre otras capacidades. C13 G incorpora las formas de los ejemplos completos como diseño, sin afirmar soporte del prototipo. Por tanto, **ninguna candidata completa de este ADR se presenta como el lenguaje que hoy acepta el ejecutable**. La descripción de separación lexer/parser/impresor existente procede de AC §9.6; esta entrega documental no audita ni modifica `src/`.

## 2. Invariantes comunes y convención de nombres

### 2.1. Reglas de integración

1. Una idea tiene una construcción semántica y una representación canónica por candidata. Una abreviatura solo se admite con expansión única, local, sin consultar tipos ni efectos, y con criterio único del impresor para escogerla. No se añaden dialectos simultáneos a la distribución final.
2. Un símbolo tiene un trabajo documentado. Se distinguen delimitadores de usos semánticos: los paréntesis delimitan agrupaciones/listas; eso no permite convertir `?` a la vez en ausencia, acceso opcional y propagación. Las sobrecargas de las tres candidatas se registran en §6.2 como deuda medible, no como prueba de cumplimiento perfecto.
3. El mérito se argumenta mediante lectura, regularidad de análisis, errores, reparación y coste visual. «Es de Rust», «parece C#» y «es fácil de implementar en Rust» no puntúan. La convención visual solicitada se adopta por AC §9.6; su efecto sobre comprensión sigue sujeto a medición.
4. No se cambia propiedad, exclusividad, orden de evaluación, limpieza, efectos, identidad nominal, exhaustividad o API para hacer más atractivo un ejemplo. El orden escrito de argumentos se mantiene antes de asociarlos a parámetros (R D10; A2 §4.2).
5. No hay retorno implícito de función/closure. `return` abandona el invocable; `value:` entrega una rama; `yield` entrega un elemento y suspende el generador síncrono (R D01/D10; C11 §1; C13 §§1–2). Una abreviatura de selección no autoriza una última sentencia implícita.
6. Lectura ordinaria presta; contexto propietario copia lo copiable y exige transferencia escrita de un binding afín. No se elige copiar o mover según conveniencia del tipo. Capturas, futuros y receptores conservan las mismas obligaciones (C11 §§2–5; C12 §§2–3).
7. Condiciones exclusivamente booleanas, efectos transitivos y límites públicos explícitos. Ninguna palabra nueva certifica por sí misma ausencia de reservas, independencia o seguridad de suspensión (R P02–P06, D08; C11 §5.2; C12 §§4–9).

### 2.2. Decisión propuesta de mayúsculas, común a A/B/C

| Categoría resuelta | Escritura canónica | Ejemplos |
|---|---|---|
| Tipos, traits, variantes, parámetros de tipo | PascalCase | `Particula`, `Result`, `I64`, `TableLayout`, `Completed`, `T` |
| Funciones y métodos declarados, públicos o privados | PascalCase | `Main`, `Responder`, `TryAdd`, `Next` |
| Campos y constantes públicos | PascalCase | `Method`, `Path`, `MaxSize` |
| Variables, parámetros, bindings de patrones y closures almacenadas | camelCase | `total`, `maxConnections`, `sumar`, `value` |
| Campos privados y constantes privadas | camelCase | `estado`, `vx`, `maxSize` |
| Segmentos de módulos/paquetes | minúsculas; varias palabras en snake_case | `std.console`, `mi_juego` |
| Palabras del lenguaje y etiquetas de contratos | grafía de la gramática | `return`, `no_alloc`, `result_from` |

Una variable que contiene una función sigue siendo variable: `let sumar = ...; sumar(2)`. Una etiqueta de argumento reproduce el parámetro; una etiqueta de construcción reproduce el campo, con su visibilidad. No se deduce una categoría por la primera letra. Los nombres siguen siendo sensibles a mayúsculas y se resuelven por declaración, conforme al espacio nominal de C13 §§8–12; este ADR no introduce resolución insensible a caja.

Para comparar sin confundir gramática y estilo, las tres candidatas reciben esta **capa ortográfica propuesta**. A conserva la gramática actual de referencia, no cada lexema histórico: `i64/u32/u64/f32/bool` pasan a `I64/U32/U64/F32/Bool`, `main` a `Main`, nombres de función a PascalCase y etiquetas a camelCase. Los intrínsecos conservan su identidad, anchura y contratos. `()` sigue siendo el tipo/valor unidad; no se añade un alias `Unit` ni un tipo `Int`. Estas grafías y el reconocimiento de `Main` como entrada son cambios futuros explícitos, no capacidades atribuidas a A1/A2. Los originales de R permanecen intactos como fixtures históricas separadas.

### 2.3. Cómo lo exige el formateador sin cambiar semántica

El formateador oficial tiene dos operaciones conceptuales, con estos nombres de interfaz **propuestos**, todavía no comandos existentes:

- `fmt`: formato de trivia y puntuación; conserva el árbol sintáctico, nombres y significado. Informa incumplimientos de nombres. `fmt --check` falla ante formato o convención incumplidos: así la convención es exigible sin que el compilador rechace un nombre por su caja.
- `fmt --fix-names`: refactorización semántica transaccional dentro del mismo producto. Primero resuelve el programa completo; cambia declaración y todas sus referencias, etiquetas y referencias dentro de interpolaciones por identidad de símbolo; después recompueba. No es una sustitución de texto ni una operación del impresor puro.

El cambio automático solo se aplica si se prueba equivalencia del grafo resuelto, tipos, modos, efectos, constantes y observables, admitiendo únicamente alfa-renombrado privado. Deben conservarse bytes de cadenas/comentarios, nombres serializados, enlaces externos y documentación con código no analizado. Si el nombre de un campo afecta derivación JSON, FFI, un registro por nombre o una exportación, no es una refactorización privada segura. Una API pública o etiqueta pública cambia el contrato (R D10; C13 §§9–12): el formateador la señala, pero **no la renombra automáticamente**. La migración de API exige un cambio explícito de versión/consumidores fuera de `fmt`; hasta entonces el estilo queda incumplido, nunca corregido silenciosamente.

Colisión `leer_archivo`/`LeerArchivo`, sombreado nuevo, código incompleto, dependencia fuera del conjunto editable o colisión de ruta entre plataformas bloquean la transacción completa. No se añade un sufijo arbitrario para lograr formato verde. Tampoco se renombra un módulo o archivo por formatear. En el ejemplo EX02 el campo privado `estado` conserva nombre y clave JSON.

Conversión determinista de nombres ASCII heredados: separar `_`, cambios minúscula→mayúscula y el último carácter de una secuencia de mayúsculas seguido por minúscula; palabras en minúscula, inicial de cada palabra en mayúscula para PascalCase, inicial de la primera en minúscula para camelCase. Acrónimos son palabras: `HTTPServer`→`HttpServer`, `GET`→`Get`. Dígitos permanecen en su palabra (`I64`, `U32`); nombres vacíos, iniciales inválidas o colisiones se diagnostican. Se conserva el alfabeto ASCII de A2 §3.3/C13 G.1; ampliar identificadores Unicode necesita decisión propia. Se prueban idempotencia, colisiones y renombrado de referencias; no basta cambiar la declaración.

**Excepción explícita por identidad de API:** las variantes de layout históricas `SoA` y `AoS` se mapean a `Soa` y `Aos` antes de aplicar la regla general. Esta última por sí sola produciría `SoA` y `AoS`; la excepción no alcanza símbolos de usuario homónimos.

Prueba obligatoria `test_conversion_corpus_completo`, antes de congelar adaptadores: inventariar **todas las ocurrencias de identificadores** de los 24 bloques de §§7–8 (incluidas interpolaciones), con candidata, archivo, span, identidad, categoría, grafía histórica si existe y grafía esperada transcrita manualmente. La igualdad entre el conjunto de spans del lexer y las filas del inventario exige cobertura del 100%, sin filtrar solo nombres que cambian. Las etiquetas contextuales y los módulos tienen filas de conservación; los símbolos nuevos también tienen esperado explícito. Para cada fila comprobar conversión, resolución a la misma identidad y segunda conversión idempotente; incluir las dos excepciones y homónimos privados sin excepción. El esperado no procede del conversor. La comprobación del grafo resuelto es una obligación del futuro arnés, no un test ya ejecutado del producto.

Test documental ejecutable con Python desde la raíz: comprueba la regla de conversión y la conservación de **cada ocurrencia** del corpus actual frente a un inventario manual de 173 grafías. El extractor solo cubre los comentarios y las cadenas/interpolaciones simples presentes aquí; no sustituye al lexer ni a la prueba de identidad/spans anterior. Las categorías se transcriben, no se infieren de la caja. Cada ocurrencia del corpus con entrada en `historical_rows` se comprueba desde su grafía histórica hasta el esperado actual; las demás tienen fila de conservación. Se exige que todas las filas históricas aparezcan en el corpus. La tabla léxica agrupa grafías coincidentes (por ejemplo, `I64` tipo e intrínseco); no demuestra por sí sola identidad semántica ni reescribe las fuentes: la tabla por span/identidad exigida arriba deberá distinguirlas y conservar `drop`, `text`, `json` y `method` ordinarios según su declaración. Los nombres de módulos y palabras de contrato se conservan; el inventario incluye también palabras clave para detectar omisiones de extracción.

```python
from pathlib import Path
import re

pascal = set("""Aos Avanzar Bind Bool Calcular Cancelled Completed Cuadrados
Dato Descartar Doble Drop Encode Err Error F32 FromValues Future Get I64
IsEmpty Json JsonEncode L Leer Lines Main Marcar Medida Method Ok Padre Particula
Path Positional PrepareUnique Println ReadText RequestView Responder Response
Result Row Salud Serve Soa Spawn String T Table TableLayout Text Trabajo Trim
TryAdd U32 U64 UniqueTableView Valor""".split())
camel = set("""activo base body capacity dato default dt e estado executor field
grupo h handler idleTimeoutMs inactivo cuenta layout limite line maxBodyBytes maxConnections
maxHeaderBytes n outcome p path pendiente ps r req resultado self server sumar
table temporal text total value view vx wide x""".split())
fixed = set("""and app as async await calculo cancel captures case cli clone console
consume continue copy create derive do drop edit else end false fn for fs function gen
http if impl import in inout items json let local match mod module move no_alloc
no_block no_io no_suspend not of own package parse pub read record region requires
resume return returns std struct then trait true until update use var with yield""".split())
expected = pascal | camel | fixed
assert len(expected) == 173
assert not (pascal & camel or pascal & fixed or camel & fixed)
exceptions = {"layout.SoA": "Soa", "layout.AoS": "Aos"}

def convert(name, style, identity=None):
    if style == "fixed":
        return name
    name = exceptions.get(identity, name)
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    name = re.sub(r"([a-z])([A-Z])", r"\1_\2", name)
    words = [word.lower() for word in name.split("_") if word]
    result = "".join(word[0].upper() + word[1:] for word in words)
    return result if style == "pascal" else result[0].lower() + result[1:]

# Filas (grafía nueva, histórica, categoría, identidad API), transcritas de §7.1.
# Esta tabla no se construye invirtiendo convert().
historical_rows = [
    ("Positional", "positional", "pascal", "cli.positional"),
    ("ReadText", "read_text", "pascal", "fs.read_text"),
    ("Lines", "lines", "pascal", "text.lines"),
    ("Trim", "trim", "pascal", "text.trim"),
    ("IsEmpty", "is_empty", "pascal", "text.is_empty"),
    ("I64", "i64", "pascal", "intrinsic.i64"),
    ("TryAdd", "try_add", "pascal", "intrinsic.try_add"),
    ("Println", "println", "pascal", "console.println"),
    ("Encode", "encode", "pascal", "json.encode"),
    ("Bind", "bind", "pascal", "http.bind"),
    ("Serve", "serve", "pascal", "http.serve"),
    ("Text", "text", "pascal", "Response.text"),
    ("Json", "json", "pascal", "Response.json"),
    ("Method", "method", "pascal", "RequestView.method"),
    ("Path", "path", "pascal", "RequestView.path"),
    ("Get", "GET", "pascal", "Method.GET"),
    ("FromValues", "from_values", "pascal", "Table.from_values"),
    ("PrepareUnique", "prepare_unique", "pascal", "Table.prepare_unique"),
    ("Row", "row", "pascal", "UniqueTableView.row"),
    ("maxConnections", "max_connections", "camel", "http.max_connections"),
    ("maxHeaderBytes", "max_header_bytes", "camel", "http.max_header_bytes"),
    ("maxBodyBytes", "max_body_bytes", "camel", "http.max_body_bytes"),
    ("idleTimeoutMs", "idle_timeout_ms", "camel", "http.idle_timeout_ms"),
    ("Soa", "SoA", "pascal", "layout.SoA"),
    ("Aos", "AoS", "pascal", "layout.AoS"),
    ("Drop", "drop", "pascal", "intrinsic.drop"),
]
historical = {new: (old, style, identity) for new, old, style, identity in historical_rows}
assert len(historical) == len(historical_rows)

def test_conversion_corpus_completo():
    document = Path("docs/adr/F0-D14-sintaxis-superficie.md").read_text(encoding="utf-8")
    fence = chr(96) * 3
    programs = re.findall(fence + r"nexo\n(.*?)" + fence, document, re.S)
    assert len(programs) == 24
    occurrences = []
    def embedded_expression(match):
        token = match.group(0)
        if token.startswith("//"):
            return ""
        return " ".join(re.findall(r"(?<!\{)\{([^{}]*)\}(?!\})", token))
    for program in programs:
        code = re.sub(r'//[^\n]*|"(?:\\.|[^"\\])*"', embedded_expression, program)
        occurrences.extend(re.findall(r"[A-Za-z_][A-Za-z_0-9]*", code))
    assert set(occurrences) == expected, (set(occurrences) - expected, expected - set(occurrences))
    seen_historical = set()
    historical_occurrences = 0
    for name in occurrences:
        style = "pascal" if name in pascal else "camel" if name in camel else "fixed"
        old, row_style, identity = historical.get(name, (name, style, None))
        assert row_style == style
        assert convert(old, style, identity) == name, (old, name, identity)
        assert convert(convert(old, style, identity), style, identity) == name
        assert convert(name, style) == name
        if name in historical:
            seen_historical.add(name)
            historical_occurrences += 1
    assert seen_historical == set(historical)
    assert historical_occurrences > len(historical_rows)
    for old, identity, wanted in (
        ("SoA", "layout.SoA", "Soa"), ("AoS", "layout.AoS", "Aos"),
        ("SoA", "user.SoA", "SoA"), ("AoS", "user.AoS", "AoS"),
        ("HTTPServer", None, "HttpServer"), ("GET", None, "Get"),
        ("read_text", None, "ReadText"), ("prepare_unique", None, "PrepareUnique"),
    ):
        assert convert(old, "pascal", identity) == wanted
        assert convert(wanted, "pascal", identity) == wanted
    assert convert("max_connections", "camel") == "maxConnections"
    print("conversion_corpus:", len(programs), "bloques;", len(occurrences),
          "ocurrencias;", historical_occurrences, "con fila historica")

test_conversion_corpus_completo()
```

## 3. Núcleo común, léxico y frontera de las candidatas

### 3.1. Significados que no varían

Las tres superficies se traducen a los mismos nodos: declaración, binding, llamada con modos, acceso a lugar, transferencia, rama con entrega explícita, retorno, propagación de `Result`, captura, futuro, región y módulo. El AST normalizado conserva distinciones de método frente a campo invocable y declaración async frente a closure async (C11 §5.1; C13 G.1). No se borra información necesaria para verificar esas reglas.

`Result<T,E>` (C: `Result of[T,E]`) y `Option<T>` son tipos nominales. `Ok`, `Err`, `Some`, `None` son constructores resueltos, no excepciones ni valores mágicos por capitalización. `?` opera solo sobre `Result`; la conversión de errores debe estar declarada, como en `app.Error` de R §0.2. No se añade conversión universal ni se modifica el perfil reducido de A2 §7. Una llamada sin `?` conserva su `Result`. No hay `T?`, `?.`, `??`, `fails`, `fail` ni `try` como mecanismos alternativos de estas candidatas completas.

Capturas sin lista: préstamo de lectura de todos los libres. Lista presente: exhaustiva, sin duplicados, sobre bindings completos; conserva entradas no usadas, orden, mutabilidad de campos propietarios y limpieza (C11 §§2–4). Los modos de invocación conservan `Call/CallMut/CallOnce`; una closure async es consumidora incluso con lista vacía (C11 §5; C12 §2).

Async distingue construir, avanzar, cancelar y destruir; `await` es exclusivo de cuerpo async; un futuro nombrado requiere transferencia. `await F()?` significa `(await F())?`. Un handle produce `TaskOutcome<T>` y exige `match` antes de propagar su payload `Result`. `region` sigue reuniendo hijas antes de liberar sus dependencias; no aparece `taskgroup`, `detach` ni `block_on` implícito (C12 §§2–8). `gen` conserva la semántica síncrona D10, incluida suspensión en `Next`, sin retorno final con valor (C13 §1.1).

Módulos: un archivo por identidad, raíz explícita, privacidad por defecto, exportación `pub`, imports sin efectos, activación explícita de extensiones y coherencia global nominal; misma revisión única de paquete y mismos contratos públicos (C13 §§8–12). Cambiar `use` por `import` en C no añade búsqueda de archivos ni acceso transitivo.

### 3.2. Reglas léxicas compartidas propuestas para la comparación

UTF-8; offsets sobre bytes originales; CRLF es un salto lógico de dos bytes, como A2 §3.1. Comentario `//` hasta fin de línea; `///` es el mismo comentario con destino documental. No se añaden comentarios de bloque. Cadenas `"..."`, escapes `\"`, `\\`, `\n`, `\r`, `\t`; `{{`/`}}` son llaves literales; `{Expr}` interpola una vez, de izquierda a derecha. Interpolación conserva el AST de su expresión y sus efectos; no permite instrucciones. `String` no admite índice entero como carácter (R D01/D10). Las reglas concretas de escapes aquí enumeradas completan el corpus de comparación, no se atribuyen al lexer reducido de A2. Cadenas raw, triples y plantillas de UI de EXT §18 quedan fuera de esta selección.

Literales decimales de los ejemplos: dígitos, o dígitos-punto-dígitos; signo separado; no se absorbe un punto seguido por otro punto (A2 §3.2). No se redefine aritmética ni redondeo. Arrays, índices, agrupación y unidad se conservan. Se conserva la semántica y el tipo esperado de los literales de array; no se adopta que `[...]` implique automáticamente `List` dinámica (EXT §12).

Precedencia, de más fuerte a más débil: selectores/llamada/índice; modo de acceso; `await`; `?`; negación numérica/lógica; multiplicación/división/resto; suma/resta; rango; comparación; conjunción; disyunción; selección `if`/`match`. Comparaciones y rangos no encadenables; operadores aritméticos y lógicos binarios asociativos a izquierda; conjunción/disyunción cortocircuitan. `not`/`!` requieren paréntesis para negar una comparación completa. Asignación es sentencia. `?` no se repite sin agrupación de una expresión nueva. Las capas no especificadas en las gramáticas reducidas de A2/C13 son una **concreción propuesta para comparar**, no soporte actual. La precedencia de modos/await/? conserva C12 §3 y C13 G.1.

Genéricos A/B usan los contextos de tipos de C13 G.3. Para una aplicación genérica en posición de expresión, este ADR propone exigir adyacencia `Nombre<T>(...)` o `Nombre<T>.Miembro`; el parser reconoce la lista por gramática de tipos y cierre seguido de `(`/`.` sin consultar símbolos. Comparaciones se escriben con espacios en formato canónico; si se quiere comparar una forma léxica indistinguible, se agrupa el operando. Esta decisión de desambiguación es nueva, debe entrar en corpus adversarial, y no se atribuye a A2. C elimina ese conflicto mediante `of[...]`. Se tokenizan `>` separados para cierres anidados en contexto genérico; `>=` sigue comparación fuera de él. Se conserva `layout: Soa` como argumento constante nombrado del tipo de EX03, sin inferir un tipo por su mayúscula.

### 3.3. Contratos y uniformidad con límites explícitos

Se mantiene la única gramática de `WithClause` de C11 §5.2: lista simple de restricciones o mapa `with { create(...), copy(...), call(...), resume(...), cancel(...), drop(...) }`, con solo las operaciones admisibles en cada contexto. Operaciones obligatorias, `unrestricted`, precedencia de diagnósticos y procedencias `from(...)`, `result_from(...)`, `retains(..., in: ...)` no cambian. Las palabras del mapa son etiquetas de operación, no funciones sujetas a PascalCase. En C solo cambia la notación de tipos y parámetros dentro de cotas; el mapa sigue delimitado por llaves, que allí sirven exclusivamente a mapas de metadatos.

La lista simple tiene significado dependiente del invocable: `call` síncrono, `resume` async privado (C11 §5.2; C12 §4). Se conserva por compatibilidad semántica documentada, y el impresor de interfaces siempre emite mapa explícito. No se inventa que `with no_alloc` cubra construir/cancelar/destruir un futuro. No se añaden `pure` o `@NoAlloc` paralelos.

### 3.4. Justificación funcional y coste a contrastar por familia

| Familia | Mérito buscado, independiente del origen | Coste o alternativa que se mide |
|---|---|---|
| Declaraciones y nombres | Distinguir binding modificable, dato y operación; reconocer firmas públicas. | Confundir función declarada con variable invocable; longitud de anotaciones. |
| Bloques y terminadores | Límites recuperables localmente al editar un programa incompleto. | Inserción por saltos A/B frente a `end` y `;` C; no se supone que las llaves ganen. |
| Modos | Ver en el punto de llamada modificación y transferencia; separar permiso de propiedad. | Palabras cortas especializadas A frente a vocabulario B/C y repetición en firmas. |
| Selección y retorno | Distinguir valor de rama de salida del invocable sin inferencia contextual oculta. | `value:` A/C frente a expansión local de selección B; errores de destino. |
| Result | La firma y el punto de propagación muestran fallo recuperable sin canal paralelo. | Anidación de tipos y omisión accidental de `?`; longitud no basta para decidir. |
| Rangos | Conocer exclusión del extremo al leer el recorrido. | Brevedad `..` A frente a extremo gráfico B y palabra C. |
| Closures | Predecir retención, copia e invalidación a partir de una lista local. | Longitud de lista y modos; abreviatura heredada A frente a solo bloque B/C. |
| Async | Distinguir construir, ejecutar, reunir y cancelar; no inferir tarea por una llamada. | Longitud de contratos y manejo explícito de outcome. |
| Módulos | Encontrar declaraciones y exportaciones sin orden oculto de imports. | Grupos, rutas y nombres locales; palabras A/B frente a C. |
| Efectos | Saber qué operación puede reservar, bloquear, suspender o hacer E/S. | Mapas completos públicos y lista simple heredada; errores de operación. |
| Texto/comentarios | Orden de interpolación visible y comentarios preservables al formatear. | Escapes, llaves literales y desplazamientos UTF-8. |
| Lógica/cotas | Operaciones booleanas y requisitos tienen composición explícita. | Símbolos A frente a palabras B/C; sobrecarga de `+` en A. |

Las justificaciones son hipótesis del diseño. Las columnas de coste determinan preguntas y negativos de §9; ninguna se puntúa por recordar a un lenguaje conocido.

## 4. Candidata A — Referencia documental más capa ortográfica

### 4.1. Definición y deuda visible

A es la combinación documental A1→A2 con C11–C13 y formas de R EX01–EX04. Se aplica únicamente la capa de nombres de §2.2 y las concreciones léxicas de §3.2. No se vuelve al `move` de A1 ni se atribuye a A2 la ejecución de lo diferido. Es el control para medir el valor de cambiar la gramática.

| Familia | Forma A y regla |
|---|---|
| Declaración | `fn F(x: I64) -> I64 { ... }`; `let`/`var`; `const C: T = comptime E`; `type Alias = T`; `struct`, `enum`, `trait`, `impl Trait for T`, `extend T`, `pub`, según C13 G.3. |
| Bloques/terminadores | Llaves; `;` o inserción exactamente C13 §§2–2.1. Cierre en misma línea no termina sentencia. Retorno seguido por salto no captura el operando posterior. |
| Paso | Lectura ordinaria `x: T`/`F(x)`; `inout x: T`/`F(inout x)`; `consume x: T`/`F(consume x)`. Receptor: `inout x.M()`/`consume f()`. |
| Valores de control | `if c { value: a; } else { value: b; }`; `match x { Some(n) => { value: n; }, None => { value: 0; } }`. Como sentencia, `if` no recibe `value:`. |
| Errores | `Result<T,E>`, `return Ok(v)`, `return Err(e)`, `F()?`. |
| Rangos | `lo..hi` semiabierto, como R EX04. Solo esta forma en A; inclusivo no se inventa como existente. |
| Closures | `fn [copy a](x: T) -> T { return a + x; }` o la abreviatura C11 `=> return a + x`. Modos `borrow/copy/inout/consume`. |
| Async/generador | `async fn`, `await consume f`, `r.Spawn(F())`, `gen fn`, `yield`, `region r(capacity: 0, executor: local) { ... }`. `Spawn` es la grafía propuesta de `.spawn(` de C12 §3; no se propone aquí una API de solicitud de cancelación. |
| Módulos | `mod calculo`; `use std.console`; `pub use package.calculo.Doble`; `use extension paquete.modulo.Tipo`. No declaración `module` de identidad. |
| Efectos/contratos | §3.3 sin cambios; `T: A + B` entre requisitos nominales, conforme C13 G.3. |
| Lógica/texto/comentarios | `&&` y `||` del diseño completo R D08; `!` como negación es concreción propuesta aquí. Cadenas/interpolación y `//` según §3.2. Su aceptación no se atribuye al perfil A2 §3.2, que excluye varios de esos tokens. |

El impresor A conserva la inserción segura, prefiere una sentencia por línea y cuerpos de closure largos. Las formas cortas siguen siendo entradas compatibles de C11. Deuda evaluable: `+` hace suma y conjunción de cotas; `=>` inicia un brazo y una abreviatura de closure; `< >` delimitan tipos y comparan valores; llaves delimitan cuerpos, imports y mapas. A no obtiene gratuitamente el máximo del criterio de coherencia. Sirve de control precisamente porque cambiar esa deuda tiene costes y beneficios que medir.

## 5. Candidata B — Propuesta externa adaptada y acotada

### 5.1. Definición completa por delta sobre A

Toda producción de A no sustituida aquí se conserva. No se importan implícitamente las demás construcciones de EXT.

| Familia | Forma B y traducción única al núcleo |
|---|---|
| Declaración/bloques | `fn`, `struct`, `enum`, `trait`, `impl`, `extend`, `let/var/const/type`, llaves y terminadores A. Parámetro nombrado: `nombre: Modo T`, con modo obligatorio. |
| Paso | `x: view T` → lectura; `x: edit T` → inout; `x: move T` → consume. En llamadas: `F(x)`, `F(edit x)`, `F(move x)`. Receptor explícito en método: `self: view`, `self: edit` o `self: move`; su tipo procede del trait/impl/extend, igual que `self` en C13 G.3. No se añade un tipo `Self`. |
| Invocación | `edit x.M()` y `move f()` mantienen la selección de receptor de C13 G.1; `(x.campo)()` sigue invocando un campo. `move` no se omite en una transferencia de binding por existir `move` en la firma. |
| Condición sentencia | `if c { ... } else { ... }`, `while`, `for`, `for edit p in ps`, `break/continue`. |
| Condición expresión | `if c then a else b`, o `if c then { sentencias; value: a; } else { sentencias; value: b; }`. `else` obligatorio. Ambas ramas son operandos de una única producción, véase §5.2. |
| Patrones | `match x { P => { value: E; }, Q => { value: F; } }`; `edit/move` sustituyen `inout/consume` en scrutinee y bindings. No brazos `P => E`. |
| Result/Option | §3.1: `Result`, `?`, `Option` explícitos; sin `fails`, `try`, `fail`, `T?` o acceso opcional. |
| Rangos | `lo..<hi` semiabierto; `lo..=hi` inclusivo. `..` solo se rechaza con reparación de extremo explícito. |
| Closures | Solo `fn [view a, copy b, edit c, move d](x: view T) -> T { return ...; }`. Lista omitida conserva captura de lectura inferida. Parámetros locales pueden omitir tipo si la inferencia C11 es única; el modo se escribe `x: view` en ese caso. |
| Async/generador | `async fn`, `await move f`, `region`, `r.Spawn(F())`, `gen fn`; no `iter fn` ni `await taskgroup`. `await F()?` conserva precedencia A. |
| Módulos | Exactamente A/C13; no `module ruta`, `private` ni imports implícitos de EXT. |
| Efectos/cotas | §3.3; requisitos `T: TraitA and TraitB`, con `and` como conjunción tanto lógica como de requisitos, sin suma. `@derive` se conserva; no `pure`, `@NoAlloc`, propiedades o defer. |
| Lógica/strings/comentarios | `and/or/not` sustituyen `&&/||/!`, sin aceptar ambos vocabularios. Cadenas y comentarios §3.2. |

En parámetros B `view/edit/move` son **modos de parámetro**, no nuevos tipos de referencia. Un tipo `T` fuera de esa producción no acepta prefijo `view`. `move` es la única palabra de propiedad transferida en firma, llamada, captura y patrón; no se adopta `take` como modo ni alias. `copy` como modo solo aparece en capturas, como en C11 §3.1; como etiqueta de operación aparece también en contratos (§6.2). La copia ordinaria sigue siendo el inicializador propietario. No se adopta un segundo operador general de copia de EXT §15.

La misma sustitución se aplica a parámetros de cotas invocables: `Call(view I64) -> I64 with { call(no_io) }`, o parámetro nombrado `Call(x: view I64)`; las cuatro operaciones públicas siguen siendo obligatorias. En B los tokens `and`, `or`, `..<` y `..=` continúan una expresión a través de salto como los operadores de A; `then`/`else` continúan la selección estructural. Los saltos tras `return` y `value:` conservan la excepción de C13 §2. Estas continuaciones adicionales forman parte del delta, no dependen de tipos.

### 5.2. Selección de valor sin retorno implícito

```text
IfExprB = "if" Condition "then" BranchOperand "else" BranchOperand
BranchOperand = NonSelectionExpr | "(" Expr ")" | BranchBlock
BranchBlock = "{" Statements FinalBranchExit "}"
FinalBranchExit = "value" ":" Expr Terminator | ReturnOrDivergence
```

`NonSelectionExpr` admite las operaciones de precedencia superior a selección de §3.2; un `if`/`match` anidado requiere paréntesis. Un operando simple se expande a una rama cuyo único nodo es `value:`. Un bloque de rama no es una expresión general utilizable fuera de esta posición; conserva la comprobación de destinatario directo y de caminos de C13 §1. Se permiten combinaciones simple/bloque sin crear una segunda forma de control. El impresor emite operando simple exactamente cuando la rama consta solo de un `value:` sin comentario asociado; en otro caso conserva bloque. Un comentario no se pierde para abreviar.

No existe `if c { ... } else { ... }` en posición expresión B: se exige `then`, incluso con bloques. No existe `=> expr` de función o closure. Son **sustituciones de superficie de C11 §1 y C13 §§1–2**, no afirmaciones de compatibilidad textual. Se conserva `return` en el código escrito de todo invocable; la selección es una expresión con operandos explícitos, no una expresión final de bloque. `value` sigue siendo identificador ordinario y `value:` marcador contextual; `let value = 3` sigue siendo válido. Los diagnósticos semánticos de destinatario/rama siguen aplicándose a los nodos expandidos, señalando el operando original.

### 5.3. Resolución expresa de las objeciones de AC §9.6

| Objeción | Resolución B y motivo propio |
|---|---|
| Método y tubería duplican composición; se pierde modo | Se elimina `\|>`; solo llamada y acceso con punto, con marca de receptor/argumento. `edit xs.M()` no se convierte en `M(xs)`. La calificación explícita de trait/extensión de C13 §10 sigue siendo desambiguación, no una tubería adicional. |
| `fails` frente a `Result` | Se conserva un único tipo de resultado escrito y un único propagador `?`; no se añade envoltura `Ok` implícita. Facilita inspeccionar firmas y almacenar el resultado. |
| `prop`/`defer` son capacidades nuevas | Fuera de B. Se usa método con llamada visible y limpieza de recursos existente. Una futura propuesta semántica debe justificar ambas separadamente. |
| Comas finales | No se amplían listas de argumentos, parámetros, capturas, tipos, arrays o imports. A2 §3 y C13 G no admiten coma final en ellas. C13 G.2/G.3 sí admite coma final en brazos, enums y separador final en campos: se conserva exactamente esa excepción heredada, y el impresor la omite. No se afirma que toda coma final esté hoy prohibida. |
| `=> expr` contradice retorno explícito | Se retira toda abreviatura de cuerpo en B; closure y función tienen bloque y `return`. `=>` queda únicamente para brazos de match. |
| `capture [...]` frente a D11 | Se conserva la posición `fn [lista]`, cambiando solo nombres de los modos. No se añade `capture` ni captura implícita por copia. |
| `if then` frente a `value:` | Expansión local definida en §5.2; bloques de rama conservan marcador final. No se vuelve palabra reservada `value`, ni se acepta retorno implícito. |

Los rangos inclusivos son una ampliación de superficie y dominio respecto al único rango ejemplificado en R EX04. Se exige implementación por comparación de extremo, sin calcular `hi + 1` que desborde; `lo > hi` produce recorrido vacío y ambos extremos tienen el mismo tipo entero. Si la representación actual no expresa inclusividad, añadir esa marca al nodo Range es una ampliación acotada del AST, no un cambio oculto de aritmética. Las tres candidatas se comparan en tareas de rangos semiabiertos; el inclusivo se evalúa adicionalmente sin dar puntos de cobertura que A no pueda obtener.

## 6. Candidata C — Cabeceras verbales y cierres explícitos

### 6.1. Definición completa por traducción estructural

C propone otra organización de fuente, no una mezcla opcional con A/B. Conserva semántica de §3.1. Se eligen palabras para cabeceras, cierres `end` y `;` obligatorio. La indentación solo formatea. Es un cambio explícito de D01 y C13 §2 en superficie, autorizado como alternativa a estudiar por AC §9.6.

| Familia | Forma C; equivalencia con A |
|---|---|
| Función | `function F(read x as I64) returns I64 do ... end`; equivale a `fn F(x: I64) -> I64 { ... }`. `returns` puede omitirse en privados bajo las mismas reglas de inferencia. |
| Declaraciones | `let/var x as T = E;`; `const c as T = comptime E;`; `type Alias = T;`; `record` por `struct`; `enum`, `trait`, `impl`, `extend` conservan su papel con `do ... end`. |
| Tipos/cotas | `Result of[I64, I64]`, `Table of[Particula, layout: Soa]`; parámetros genéricos `of[L requires TableLayout]`; conjunción `and`; `where T requires A and B`. `dyn (...)`, cotas invocables y Future mantienen sus contratos con esta notación de tipos. |
| Bloques/terminadores | `do ... end`; toda sentencia simple, import, módulo, alias, const, campo, variante o requisito sin cuerpo termina en `;`. Declaración/sentencia estructural termina en `end`; cuando es una expresión, la sentencia que la contiene requiere `;`. Saltos nunca insertan terminadores. |
| Paso | `read x as T`, `update x as T`, `own x as T`; llamadas `F(x)`, `F(update x)`, `F(own x)`; métodos declaran `read/update/own self` sin `as`. |
| If/match | `if c do ... else ... end`; expresión exige `value:` final por rama igual que A. `match e do case P do ... end case Q do ... end end`; guardas `case P if c do`. Sin flechas ni `then`. |
| Result/propagación | `Result of[T,E]`; `Ok/Err`; `?` único, precedencia §3.2. |
| Rangos | `lo until hi`, solo semiabierto. `until` tiene la precedencia de rango y no es bucle. No se añade rango inclusivo a C en este protocolo. |
| Closure | `function captures(read a, clone b, update c, own d)(read x as T) returns T do return E; end`. `captures(...)` opcional; lista vacía explícita `captures()`. Solo cuerpo largo. |
| Async/generador | `async function`, `await own f`, `gen function`, `yield E;`; `region r(capacity: 0, executor: local) do ... end`; `r.Spawn(F())`. |
| Módulos | `module calculo;` declara subarchivo como `mod calculo`; `import ruta;` por `use`; `pub import`, `import extension`; raíz sigue en manifiesto, no en declaración de identidad. Grupo `import paquete.modulo items(A, B);`. |
| Efectos/procedencia | `with` y mapas de §3.3; `from/result_from/retains` conservados. `@derive(JsonEncode)` conservado. `with ps.PrepareUnique() as view do ... end` conserva el ámbito de guarda de EX03. |
| Strings/comentarios | §3.2; operadores lógicos `and/or/not`. `for update p in ps do ... end`, `while c do ... end`, `break;` y `continue;`. |

`as` asocia un tipo en declaración o un binding en import/guarda: nunca convierte un valor. `:` queda para asociación etiqueta→operando, incluido `value:`; no anota tipos. `of[...]` delimita aplicaciones de tipos; `[]` sin `of` mantiene arrays/índices. Llaves quedan en mapas de contratos e interpolación, con contextos léxicos distintos, nunca como cuerpo alternativo. `+` solo suma; `=>` y `->` no existen en C. Una firma de cota se escribe, por ejemplo, `Call(read I64) returns I64 with { call(no_io) }`; parámetro nombrado `Call(read x as I64)` permite `from(x)`. Estas reglas sustituyen uniformemente las producciones de C11 §5.2 y C13 G, sin eliminar requisitos de cotas públicas.

La copia de captura `clone b` significa exactamente copia lógica C11, no clonación arbitraria con efectos externos. Se elige palabra distinta de `copy` para no confundirla con la etiqueta de operación del mapa de efectos; no hay operador general `clone`. Métodos y campos invocables conservan la distinción `x.M()`/`(x.f)()`, y `update/own` califican el receptor de la última llamada como A. No se introducen captura por proyección, métodos dentro de `record` ni inferencia de conformidad.

`end` cierra la construcción abierta más próxima; no lleva nombre ni palabra opcional. Para usar un bloque ordinario se escribe `do ... end`. Un match tiene `end` para cada brazo y otro para el match. En closures anidadas, `return` pertenece al invocable más cercano y `value:` solo a la rama directa: el cierre verbal no altera alcance. La verbosidad y el posible error de contar `end` son hipótesis de riesgo que se medirán, no virtudes asumidas.

### 6.2. Inventario comparado de símbolos con varios trabajos

Este inventario antecede al manifiesto de medición (§9.1) y forma parte de sus entradas. «Contexto distinguible» significa parseo determinista, no ausencia de carga cognitiva. Cada fila con varios trabajos exige positivos pareados, un negativo aislado por contexto y preguntas de confusión entre trabajos en las tres cohortes; su justificación no concede automáticamente el punto de coherencia. Los usos comunes tampoco son méritos diferenciales de B.

| Símbolo | Trabajos en A | Trabajos en B | Trabajos en C | Resolución o coste declarado |
|---|---|---|---|---|
| `with` | Contrato de efectos; guarda de recurso EX03. | Los mismos dos. | Los mismos dos. | Cabecera de invocable frente a sentencia `with Expr as Name`; se conserva la sobrecarga heredada, se mide atribución incorrecta de efectos a la guarda. |
| `copy` / `clone` | `copy` modo de captura y etiqueta de operación del contrato. | Igual que A. | `clone` solo modo de captura; `copy` solo etiqueta de operación. | A/B pagan cambio de función gramatical; C paga dos vocablos para copia lógica. `clone` no autoriza clonación profunda ni efectos adicionales: medir esa expectativa por cohorte. |
| `drop` / `Drop` | `drop(...)` etiqueta de operación; `Drop(...)` intrínseco resuelto de destrucción. | Igual que A. | Igual que A. | Se conserva la distinción de caja en EX06; el mapa se analiza como metadatos, la llamada se resuelve por identidad (A2 §6.5; C12 §3). No basta reconocer texto para conceder consumo. Medir errores de caja/contexto y conservar prueba de función homónima ordinaria. |
| `:` / `value:` | Anotación de tipo, requisito genérico, argumento/campo etiquetado y entrega de rama. | Los mismos trabajos. | Etiqueta de argumento/campo y entrega de rama; tipos con `as`, requisitos con `requires`. | `value:` cambia control, no es argumento nombrado. Su posición de sentencia de rama lo distingue; A/B conservan además sobrecarga de anotación. Medir destino erróneo y positivo `let value`. |
| `view` | Nombre ordinario de guarda en EX03. | Modo de lectura en firma/captura y nombre de binding en `as view`, `edit view`. | Nombre ordinario de guarda. | B lo hace contextual (§7.4), sin reserva global. La ambigüedad visual y el esfuerzo de aprender posiciones cuentan como coste. |
| `as` | Alias de import y binding de guarda. | Igual que A. | Los dos anteriores y anotación de tipo. | Producciones distintas y sin conversión de valores; C paga una asociación adicional. Medir confusión de tipo, alias y recurso. |
| `[` / `]` | Array, índice y lista de capturas. | Igual que A. | Array, índice y aplicación de tipos `of[...]`. | El token precedente delimita la producción; el inventario no oculta el uso de C dentro de `of[`. Medir recuperación de cierre y confusión entre array y tipo. |
| `<` / `>` | Comparación y genéricos. | Igual que A. | Solo comparación. | A/B exigen desambiguación de §3.2 y pagan su regla de adyacencia; C paga `of[...]`. |
| `+` / `and` | `+` suma y conjunción de cotas; `&&` conjunción booleana. | `+` suma; `and` conjunción booleana y de cotas. | Igual que B. | B/C reúnen conjunción bajo un nombre, pero dominios y evaluación difieren: una cota no cortocircuita un efecto de ejecución. |
| `=>` | Brazo de match y cuerpo abreviado de closure. | Solo brazo de match. | Ausente. | A conserva ambas entradas C11; el impresor prefiere cuerpo largo. La retirada en B/C es un diferencial medible respecto de A. |
| `{` / `}` | Cuerpos, grupos de import, mapas de efectos e interpolaciones. | Igual que A. | Mapas de efectos e interpolaciones. | Delimitación compartida con contextos distintos; C cambia cuerpos a `do/end` e imports a `items(...)`. Medir cierres y recuperación, no atribuir pureza absoluta a C. |
| `(` / `)` | Agrupación, llamada, parámetros, constructores/patrones, unidad y contratos. | Igual que A. | Los mismos más `captures(...)` e `items(...)`. | Delimitadores comunes, no operaciones intercambiables. El CST debe conservar la producción y paréntesis desambiguadores. |
| `=` / `-` / `.` | `=` inicializa/asigna/define alias; `-` negación/resta; `.` selección/calificación de ruta. | Igual que A. | Igual que A. | Sobrecargas conservadas por contexto y aridad; registrar errores de actualización frente a binding, precedencia y resolución de ruta. |
| `inout` / `edit` / `update`; `consume` / `move` / `own` | Permiso exclusivo o propiedad en firma, uso, captura y patrón. | Mismos conceptos con `edit` y `move`. | Mismos conceptos con `update` y `own`. | Son apariciones del mismo modo, no recepción/transferencia con dos palabras. B/C añaden colisión con nombres ordinarios (§7.4); A mantiene reservas A2. |

`record` puede activar expectativas de igualdad estructural, y `clone` de copia profunda; `struct` y `copy` tampoco son neutrales por definición. El cuestionario de declaraciones y lectura/copia registra explícitamente esas predicciones, experiencia previa y resultados por cohorte. En las tres candidatas, añadir `pub` a un campo cambia la grafía canónica de `valor` a `Valor`: es coste de la convención común, también en B. La tarea de visibilidad exige actualizar referencias/etiquetas por identidad y tratarlo como cambio de API (§2.3), no como formato inocuo ni ventaja de B.

## 7. Corpus literal común: EX01–EX04

### 7.1. Contrato de fidelidad y nombres de biblioteca

Los bloques siguientes son **programas completos de diseño**, con todos sus imports, funciones y cuerpos; no sustituyen las fixtures verbatim de R. Misma biblioteca propuesta por R §0.2, sin auxiliares omitidos ni simulación encubierta. A2 §12.5 documenta por qué el perfil reducido no ejecuta los originales. EX01/EX04 siguen siendo puertas F2, EX03 F3 y EX02 F5 (R §0.3; C13 G). Una prueba de sintaxis previa puede analizar/resolver contra interfaces declaradas; eso no acredita ejecución de esas puertas.

Aplicación uniforme de §2.2: `cli.positional`→`cli.Positional`, `fs.read_text`→`fs.ReadText`, `text.lines/trim/is_empty`→`Lines/Trim/IsEmpty`, `parse.i64`→`parse.I64`, `try_add`→`TryAdd`, `println`→`Println`, `json.encode`→`json.Encode`; `http.bind/serve/Response.text/Response.json`→`Bind/Serve/Response.Text/Response.Json`; `req.method/path`→`req.Method/Path`, `Method.GET`→`Method.Get`; `from_values/prepare_unique/row`→`FromValues/PrepareUnique/Row`. Etiquetas `max_connections/max_header_bytes/max_body_bytes/idle_timeout_ms`→`maxConnections/maxHeaderBytes/maxBodyBytes/idleTimeoutMs`. `SoA/AoS`→`Soa/Aos` por las excepciones de identidad de §2.3; `drop`→`Drop` conserva el intrínseco de A2 §6.5, mientras la etiqueta de operación `drop` queda intacta. Estos cambios de API se proponen explícitamente para el corpus comparativo, con mapa de identidades uno a uno; **no los hace `fmt` sobre APIs históricas**. No se modifica formato JSON `estado`, rutas, mensajes ni contratos.

### 7.2. EX01 — Total de registros

Entrada, error de parseo/desbordamiento y limpieza: R EX01. Con `12`, `8`, `5`, salida `Total: 25`; líneas vacías ignoradas; ruta por defecto `ventas.txt`. A/B/C evalúan y propagan en el mismo orden.

**A — `total.nx`**

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

**B — `total.nx`**

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

**C — `total.nx`**

```nexo
import std.app;
import std.cli;
import std.console;
import std.fs;
import std.parse;

function Main() returns Result of[(), app.Error] do
    let path = cli.Positional(0, default: "ventas.txt")?;
    let text = fs.ReadText(path)?;
    var total as I64 = 0;
    for line in text.Lines() do
        let field = line.Trim();
        if field.IsEmpty() do
            continue;
        end
        let value = parse.I64(field)?;
        total = total.TryAdd(value)?;
    end
    console.Println("Total: {total}")?;
    return Ok(());
end
```

La igualdad literal de A y B en EX01 es deliberada: no se cambia una construcción solo para aparentar originalidad. Su diferencia se ejerce en modos, selecciones, lógica y rangos de los demás casos.

### 7.3. EX02 — HTTP/JSON local

Contrato R EX02: síncrono, ruta `/salud`, 405/404, JSON `{"estado":"ok"}`, respuesta propietaria, límites y cierre automatizado. Conserva 64 conexiones, 16384 bytes de cabeceras, 1048576 bytes de cuerpo y 5000 ms. Sin TLS ni despliegue público.

**A — `salud.nx`**

```nexo
use std.app
use std.http
use std.json

@derive(JsonEncode)
struct Salud {
    estado: String
}

fn Responder(req: http.RequestView) -> Result<http.Response, app.Error> {
    if req.Method != http.Method.Get {
        return Ok(http.Response.Text(405, "Método no permitido"))
    }
    if req.Path != "/salud" {
        return Ok(http.Response.Text(404, "Ruta no encontrada"))
    }
    let body = json.Encode(Salud(estado: "ok"))?
    return Ok(http.Response.Json(200, consume body))
}

fn Main() -> Result<(), app.Error> {
    let server = http.Bind(
        "127.0.0.1:8080",
        maxConnections: 64,
        maxHeaderBytes: 16384,
        maxBodyBytes: 1048576,
        idleTimeoutMs: 5000
    )?
    http.Serve(consume server, handler: Responder)?
    return Ok(())
}
```

**B — `salud.nx`**

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

**C — `salud.nx`**

```nexo
import std.app;
import std.http;
import std.json;

@derive(JsonEncode)
record Salud do
    estado as String;
end

function Responder(read req as http.RequestView) returns Result of[http.Response, app.Error] do
    if req.Method != http.Method.Get do
        return Ok(http.Response.Text(405, "Método no permitido"));
    end
    if req.Path != "/salud" do
        return Ok(http.Response.Text(404, "Ruta no encontrada"));
    end
    let body = json.Encode(Salud(estado: "ok"))?;
    return Ok(http.Response.Json(200, own body));
end

function Main() returns Result of[(), app.Error] do
    let server = http.Bind(
        "127.0.0.1:8080",
        maxConnections: 64,
        maxHeaderBytes: 16384,
        maxBodyBytes: 1048576,
        idleTimeoutMs: 5000
    )?;
    http.Serve(own server, handler: Responder)?;
    return Ok(());
end
```

### 7.4. EX03 — Mismo algoritmo sobre dos layouts

Contrato R EX03: primera posición `0.5`, segunda `1.5`; cambiar `Soa` por `Aos` conserva `Avanzar`. Preparar unicidad y escribir consola quedan fuera del contrato del núcleo, dentro de los costes totales. Los campos privados conservan camelCase.

**A — `particulas.nx`**

```nexo
use std.app
use std.console
use std.table.{Table, TableLayout, UniqueTableView, Soa, Aos}

struct Particula {
    x: F32
    vx: F32
}

fn Avanzar<L: TableLayout>(inout ps: UniqueTableView<Particula, L>, dt: F32)
    with no_alloc, no_block, no_suspend, no_io {
    for inout p in ps {
        p.x += p.vx * dt
    }
}

fn Main() -> Result<(), app.Error> {
    var ps = Table<Particula, layout: Soa>.FromValues([
        Particula(x: 0.0, vx: 1.0),
        Particula(x: 2.0, vx: -1.0)
    ])
    with ps.PrepareUnique() as view {
        Avanzar(inout view, dt: 0.5)
    }
    console.Println("Primera posición: {ps.Row(0).x}")?
    return Ok(())
}
```

**B — `particulas.nx`**

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

**C — `particulas.nx`**

```nexo
import std.app;
import std.console;
import std.table items(Table, TableLayout, UniqueTableView, Soa, Aos);

record Particula do
    x as F32;
    vx as F32;
end

function Avanzar of[L requires TableLayout](update ps as UniqueTableView of[Particula, L], read dt as F32)
    with no_alloc, no_block, no_suspend, no_io do
    for update p in ps do
        p.x += p.vx * dt;
    end
end

function Main() returns Result of[(), app.Error] do
    var ps = Table of[Particula, layout: Soa].FromValues([
        Particula(x: 0.0, vx: 1.0),
        Particula(x: 2.0, vx: -1.0)
    ]);
    with ps.PrepareUnique() as view do
        Avanzar(update view, dt: 0.5);
    end
    console.Println("Primera posición: {ps.Row(0).x}")?;
    return Ok(());
end
```

En B, `view` es contextual en modo de parámetro/captura; tras `as` y en argumento sigue siendo un nombre ordinario. Esta decisión evita romper la guarda llamada `view` de EX03. `copy`, `edit`, `move` también pueden ser nombres donde la producción exige Name. En posición de expresión, `edit`/`move` seguidos por un inicio de Postfix se reconocen prioritariamente como prefijo; en otro caso son Name. Por tanto `F(move)` lee un binding, `move(x)` transfiere el lugar agrupado y `(move)(x)` invoca un binding llamado move. El formateador conserva los paréntesis que desambiguan. No se consulta el tipo ni se elige por la existencia de una función homónima. En C se aplica la misma regla a `update`/`own`. El corpus de ambigüedades debe incluir estos nombres, el salto entre prefijo y operando y su diagnóstico de operando ausente.

La contextualización de `move` es un **delta léxico exclusivo de B**: A2 §3.3 lo reserva y §15 mantiene `NX-E0006`, `feature_id = legacy_move`, en cualquier posición. A conserva ese rechazo; B lo retira para sus usos de modo y nombre. No se detecta la versión por el contenido. Matriz obligatoria con versión explícita: en A, `let move = 1` y `F(move x)` producen el diagnóstico de migración sobre `move`; en B, `let move = 1; F(move)` lee el binding, `F(move x)` transfiere, `move(x)` transfiere el lugar agrupado y `(move)(x)` invoca el binding. En B, un modo sin operando en una posición que exige modo, como captura `fn [move]`, da error sintáctico de nombre faltante, nunca `legacy_move`; `F(move)` sin declaración da error de resolución, no de migración. Fijar tokens, spans y campos por superficie antes de medir, sin inventar aquí IDs nuevos de diagnóstico. Se conserva por separado el negativo de uso posterior a transferencia para que no quede oculto por una palabra rechazada.

### 7.5. EX04 — Generador síncrono

Contrato R EX04: salida `0`, `1`, `4`, `9`, una línea por elemento. Ensanchamiento exacto U32→U64; sin array materializado. Finalización anticipada y error de escritura destruyen el estado una vez. El bucle que avanza no promete `no_suspend`.

**A — `cuadrados.nx`**

```nexo
use std.app
use std.console

// Para un U32, el cuadrado cabe en U64.
gen fn Cuadrados(limite: U32 = 4) -> U64 {
    for n in 0..limite {
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

**B — `cuadrados.nx`**

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

**C — `cuadrados.nx`**

```nexo
import std.app;
import std.console;

// Para un U32, el cuadrado cabe en U64.
gen function Cuadrados(read limite as U32 = 4) returns U64 do
    for n in 0 until limite do
        let wide = U64(n);
        yield wide * wide;
    end
end

function Main() returns Result of[(), app.Error] do
    for value in Cuadrados(limite: 4) do
        console.Println("{value}")?;
    end
    return Ok(());
end
```

El parámetro de EX04 conserva el modo del original y sus obligaciones D10; materializar el escalar retenido debe justificarse por las reglas del generador, no por cambiarlo a transferencia para una sola candidata. La interfaz y los defaults de la biblioteca se prueban de forma común.

## 8. Tres programas adicionales, completos en cada candidata

### 8.1. EX05 — Captura independiente, selección y Result

Sin biblioteca externa. Con contador inicial 0, `Main` llama primero a `Calcular(false, …)` y después a `Calcular(true, …)` sobre el mismo contador. Esperados transcritos: `Ok(5)` y cuenta 0 tras la primera llamada; `Ok(8)` y cuenta 1 tras la segunda; resultado final `Ok(())`. La captura copia `base = 3`; cambiar el original a 100 no altera la closure. `Marcar` incrementa el contador exactamente una vez y retorna `true`: es el segundo operando de la conjunción y no debe ejecutarse con `activo = false`. La tercera condición conserva negación y comparación. La ruta falsa selecciona 2 y suma 3; la verdadera selecciona 5 y suma 3.

`Main` comprueba por separado resultado y contador: errores 1/2 para resultado/contador de la ruta falsa y 3/4 para los de la verdadera. Se conserva la forma de entrada de A2 §4.4 salvo la grafía propuesta. Por A2 §§4–5 y las traducciones de §§4–6, los préstamos exclusivos están escritos tanto en las firmas de `Calcular`/`Marcar` como en ambas llamadas: `inout` en A, `edit` en B, `update` en C. El préstamo termina al volver de la llamada, antes de leer el contador en `Main`; no se captura `cuenta`. Los `;` interiores de A/B son necesarios en cuerpos de una línea (C13 §2). `value` conserva uso como variable. Regresiones exigidas al futuro arnés: sustituir la conjunción por evaluación ansiosa debe alcanzar `Err(2)`; omitir o duplicar el incremento en la ruta verdadera debe alcanzar `Err(4)`. El esperado no se obtiene del programa bajo prueba.

**A — `capturas.nx`**

```nexo
fn Marcar(inout cuenta: I64) -> Bool {
    cuenta = cuenta + 1
    return true
}

fn Calcular(activo: Bool, inout cuenta: I64) -> Result<I64, I64> {
    var base: I64 = 3
    let sumar = fn [copy base](x: I64) -> I64 { return base + x; }
    base = 100
    let value = if activo && Marcar(inout cuenta) && !(base == 0) {
        let temporal: I64 = 5
        value: temporal
    } else {
        value: 2
    }
    let resultado: Result<I64, I64> = Ok(sumar(value))
    return match resultado {
        Ok(n) => { value: Ok(n); },
        Err(e) => { value: Err(e); }
    }
}

fn Main() -> Result<(), I64> {
    var cuenta: I64 = 0
    let inactivo = Calcular(false, inout cuenta)?
    if inactivo != 5 {
        return Err(1)
    }
    if cuenta != 0 {
        return Err(2)
    }
    let resultado = Calcular(true, inout cuenta)?
    if resultado != 8 {
        return Err(3)
    }
    if cuenta != 1 {
        return Err(4)
    }
    return Ok(())
}
```

**B — `capturas.nx`**

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

**C — `capturas.nx`**

```nexo
function Marcar(update cuenta as I64) returns Bool do
    cuenta = cuenta + 1;
    return true;
end

function Calcular(read activo as Bool, update cuenta as I64) returns Result of[I64, I64] do
    var base as I64 = 3;
    let sumar = function captures(clone base)(read x as I64) returns I64 do
        return base + x;
    end;
    base = 100;
    let value = if activo and Marcar(update cuenta) and not (base == 0) do
        let temporal as I64 = 5;
        value: temporal;
    else
        value: 2;
    end;
    let resultado as Result of[I64, I64] = Ok(sumar(value));
    return match resultado do
        case Ok(n) do
            value: Ok(n);
        end
        case Err(e) do
            value: Err(e);
        end
    end;
end

function Main() returns Result of[(), I64] do
    var cuenta as I64 = 0;
    let inactivo = Calcular(false, update cuenta)?;
    if inactivo != 5 do
        return Err(1);
    end
    if cuenta != 0 do
        return Err(2);
    end
    let resultado = Calcular(true, update cuenta)?;
    if resultado != 8 do
        return Err(3);
    end
    if cuenta != 1 do
        return Err(4);
    end
    return Ok(());
end
```

### 8.2. EX06 — Construir un futuro inerte y reunir una hija

Archivo completo con entrada síncrona: `Main` construye un futuro inerte, transfiere su propiedad al parámetro de `Descartar`, lo destruye sin ejecutar su cuerpo y retorna `Ok(())`. El arnés semántico, cuando exista soporte D12, invoca además `Padre` y lo avanza: ruta normal `Ok(7)`, cancelación de hija `Err(9)`, cancelación del padre inicia limpieza sin fabricar un resultado. Esas invocaciones y la inyección de cancelación son casos futuros del arnés; el programa no solicita cancelación ni demuestra esa ruta por sí solo. No se inventa una API `RunAsync`. No se atribuye soporte async al perfil A2 ni se adelanta la puerta F5 (C12, «Interacciones y alcance por fase»).

`Future<Result<I64,I64>>` designa el futuro concreto de `Padre` según C12 §2, sin boxing ni cambio de contratos. Tarea de generación EX06 «completar cuerpo con firma dada»: completar `Descartar` y la llamada propietaria en `Main`. Su negativo de modo elimina únicamente `consume`/`move`/`own` de esa llamada: intenta copiar un futuro no copiable y debe alcanzar `NX-E0201`, no un error sintáctico. Positivo pareado: marca restaurada, una transferencia y una destrucción, ninguna ejecución de `Padre`. La batería semántica adicional comprueba que un uso posterior de `pendiente` alcanza `NX-E0202/definitely_consumed`. Cada esperado, token, span y campos se transcribe por superficie antes de medir. Estas tareas ocupan las casillas ya previstas para EX06 (§9.2), no añaden familias ni sesiones.

**A — `tareas.nx`**

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
        let outcome = await consume h
        return match consume outcome {
            Completed(consume r) => { value: consume r; },
            Cancelled => { value: Err(9); }
        }
    }
}

fn Descartar(consume pendiente: Future<Result<I64, I64>>) {
    Drop(pendiente)
    return
}

fn Main() -> Result<(), I64> {
    let pendiente = Padre()
    Descartar(consume pendiente)
    return Ok(())
}
```

**B — `tareas.nx`**

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

**C — `tareas.nx`**

```nexo
pub async function Trabajo() returns Result of[I64, I64]
    with {
        create(no_alloc, no_block, no_suspend, no_io),
        resume(no_alloc, no_block, no_suspend, no_io),
        cancel(no_alloc, no_block, no_suspend, no_io),
        drop(no_alloc, no_block, no_suspend, no_io)
    } do
    return Ok(7);
end

async function Padre() returns Result of[I64, I64] do
    region grupo(capacity: 0, executor: local) do
        let h = grupo.Spawn(Trabajo());
        let outcome = await own h;
        return match own outcome do
            case Completed(own r) do
                value: own r;
            end
            case Cancelled do
                value: Err(9);
            end
        end;
    end
end

function Descartar(own pendiente as Future of[Result of[I64, I64]]) do
    Drop(pendiente);
    return;
end

function Main() returns Result of[(), I64] do
    let pendiente = Padre();
    Descartar(own pendiente);
    return Ok(());
end
```

`Drop` es la grafía canónica propuesta del intrínseco `drop` de C11 §3.1/A2 §6.5: consume por contrato sintáctico propio, no por inferencia de firma ni por empezar en mayúscula. El mapa de efectos mantiene su etiqueta `drop`. No se cambia la semántica para un usuario que declare otro nombre parecido.

### 8.3. EX07 — Módulo, trait nominal y contrato público

Dos archivos, raíz `main.nx` suministrada al compilador; no hay archivos auxiliares omitidos. `Doble(Dato(Valor: 6))` produce 12; `Main` lo comprueba y retorna `Ok(())`, con error 1 si no coincide. El trait y el tipo son públicos; el campo `Valor` también. La implementación está en el paquete propietario y las firmas publican sus cotas. `Doble` no depende de que otro tipo tenga casualmente un método homónimo (R D08; C13 §§9–11).

**A — `main.nx`**

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

**A — `calculo.nx`**

```nexo
pub trait Medida {
    fn Leer(self) -> I64 with no_alloc, no_block, no_suspend, no_io
}

pub struct Dato {
    pub Valor: I64
}

impl Medida for Dato {
    fn Leer(self) -> I64 with no_alloc, no_block, no_suspend, no_io {
        return self.Valor
    }
}

pub fn Doble<T: Medida>(dato: T) -> I64
    with no_alloc, no_block, no_suspend, no_io {
    return dato.Leer() * 2
}
```

**B — `main.nx`**

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

**B — `calculo.nx`**

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

**C — `main.nx`**

```nexo
module calculo;
import package.calculo items(Dato, Doble);

function Main() returns Result of[(), I64] do
    let dato = Dato(Valor: 6);
    if Doble(dato) != 12 do
        return Err(1);
    end
    return Ok(());
end
```

**C — `calculo.nx`**

```nexo
pub trait Medida do
    function Leer(read self) returns I64 with no_alloc, no_block, no_suspend, no_io;
end

pub record Dato do
    pub Valor as I64;
end

impl Medida for Dato do
    function Leer(read self) returns I64 with no_alloc, no_block, no_suspend, no_io do
        return self.Valor;
    end
end

pub function Doble of[T requires Medida](read dato as T) returns I64
    with no_alloc, no_block, no_suspend, no_io do
    return dato.Leer() * 2;
end
```

## 9. Protocolo reproducible de medición

### 9.1. Registro previo, herramientas y dominio de evidencia

Este es un protocolo propuesto ejecutable por el equipo mediante un arnés local; **el arnés de comparación aún debe construirse y verificarse**. No se confunde un diseño de protocolo con mediciones realizadas. Antes de recoger datos, fijar en un manifiesto versionado: hash de este ADR, variantes de programas, interfaces de biblioteca, entradas/salidas y trazas esperadas, parsers/impresores, modelos y versiones, prompts, semillas, límites, métricas, tamaños, análisis y reglas de exclusión. Congelar los tres adaptadores a la vez. No usar red ni servicios de pago sin autorización separada; la medición puede hacerse con agentes disponibles localmente, documentando sus límites.

Prerrequisitos: extracción literal de los 24 bloques de programa de §§7–8 (EX07 tiene dos archivos por candidata); manifiesto que agrupe cada programa; tres adaptadores con `parse`, `print`, `resolve/check` y, donde exista, `run`. `parse` consume hasta EOF. Los tres producen el mismo AST normalizado, salvo origen fuente y la expansión documentada de selección B. Los nombres de biblioteca propuestos resuelven al mismo conjunto de identidades mediante un manifiesto explícito. Ningún adaptador reconoce el nombre de una fixture para producir su respuesta.

Separar tres niveles en cada registro: **sintaxis**, **semántica estática**, **ejecución real**. Sin biblioteca/runtime de una familia, solo se informa lo alcanzado. Para la comparación antes de F1 se admite un oráculo estático independiente y casos reducidos ejecutables que conserven la regla estudiada, identificados como reducidos; no se sustituye EX02 completo por una tabla de respuestas para contar éxito funcional. Una familia sin comprobador común de su contrato permanece NO EVALUADA y no entra como acierto en la decisión final. El bloqueo por herramientas insuficientes puede impedir elegir, no obliga a fingir soporte F5.

### 9.2. Prueba de agentes: generación y reparación

**Muestra fijada inicialmente:** 7 familias (EX01–EX07), cuatro tareas de generación por familia, ocho negativos aislados por familia, dos modelos distintos preseleccionados y cinco semillas por modelo/tarea/candidata. Son **280 sesiones de generación y 560 de reparación por candidata; 2520 sesiones en total**. Cada sesión parte de contexto vacío. Los dos modelos no se presentan como una muestra representativa de toda IA; se reportan por separado además del agregado. Si solo hay uno disponible, el protocolo se revisa antes de medir y la limitación queda explícita.

Las cuatro tareas por familia son: escribir desde contrato, completar un cuerpo con firma dada, modificar un requisito de comportamiento y trasladar un caso válido a otra organización de datos sin cambiar su contrato. Los ocho negativos se fijan individualmente antes de ver respuestas: dos léxicos/sintácticos, uno de nombre/etiqueta, uno de tipo/Result, uno de modo/captura, uno de rama/retorno, uno de efecto/limpieza y uno del rasgo principal de la familia. En familias sin async o módulos no se fuerza un fallo de esas clases: se usa la regla principal correspondiente. Se conserva una matriz explícita tarea→regla→positivo pareado→oráculo. No se cuentan ocho variantes del mismo carácter como ocho reglas.

Cada prompt contiene el mismo contrato semántico, hoja de referencia de la candidata, interfaces necesarias y ejemplos de entrenamiento distintos de los evaluados. Se igualan contenido y capacidades, no se entrega el programa solución. Mismo límite de contexto, 6000 tokens de salida inicial y 3000 por reparación; temperatura y semilla fijadas donde la herramienta las soporte, y esa ausencia se registra donde no. Máximo tres propuestas por sesión: inicial y dos revisiones con diagnósticos JSON equivalentes. Límite de 180 s por propuesta. No se consulta red, corpus oculto ni respuestas de otra sesión. Orden A/B/C aleatorio por bloques de tarea/modelo/semilla.

Generación: compilar/comprobar primera propuesta, registrar fallos, devolver solo diagnóstico y contrato; repetir hasta éxito o presupuesto. Reparación: presentar el programa con exactamente un defecto sembrado y el mismo acceso al comprobador. No entregar la edición correcta. Un cambio que elimina una función, relaja un contrato, modifica la salida exigida o sustituye el algoritmo por constantes es fracaso, aunque compile.

Puntuar la fuente tal como la entrega el agente, antes de formatear o arreglar nombres. Registrar por separado aceptación semántica y cumplimiento de estilo; el éxito completo requiere ambos. Si usa el formateador durante la sesión, esa llamada, tiempo y edición cuentan dentro del mismo presupuesto. Un formato posterior para presentar resultados no mejora retroactivamente G1 o R2.

Métricas primarias, por candidata, modelo y familia:

- **G1**: fracción de generación correcta en la primera propuesta, según todos los niveles disponibles exigidos por esa tarea; objetivo ≥85% global y ≥75% por familia/modelo.
- **R2**: fracción de negativos reparados dentro de las tres propuestas totales; objetivo ≥90% global y ≥80% por familia/modelo.
- **E**: episodios causales nuevos por sesión; máximo medio 0,50 en generación y 0,25 en reparación después de la primera edición. Un defecto que produce 20 diagnósticos cuenta una vez; una regresión distinta cuenta otro episodio. Registrar además número bruto de diagnósticos, sin usarlo para premiar parsers que silencian cascadas.
- **S**: número de propuestas aceptadas que violan el contrato dividido por todas las propuestas aceptadas, incluidas intermedias: debe ser cero **en el conjunto finito revisado al 100%**. No es una afirmación de tasa poblacional nula. Sin aceptadas, S es NO EVALUADO, no cero. Un hallazgo invalida el oráculo afectado y las puntuaciones, no se atribuye éxito a la sintaxis.

Secundarias: tokens totales, bytes/caracteres, latencia hasta solución correcta, número de llamadas de herramienta, distancia de edición y porcentaje de arreglos aplicables que recomprueban. Se cuentan timeouts y salidas vacías como fallos; no se eliminan muestras por resultar difíciles. La clasificación separa sintaxis, nombres, tipos, propiedad, efectos, runtime y cambio indebido de contrato (R §7.3–7.5).

Para S, dos revisores independientes del implementador inspeccionan **cada propuesta aceptada** frente al contrato, fuente completa y evidencia de todos los niveles exigidos por la tarea. Pueden apoyar la revisión en un oráculo diferencial de implementación independiente, pero nunca eximir una aceptada por muestreo ni reutilizar el arnés como su propio oráculo. Guardar los dos veredictos, hash de fuente, cláusulas comprobadas y trazas; adjudicar toda discrepancia por un tercer revisor antes de puntuar. Una propuesta pendiente o inconclusa bloquea la puerta S. Tras corregir un oráculo, reejecutar y revisar todas las propuestas afectadas, no solo el hallazgo. La cobertura 100% y el numerador cero son comprobaciones separadas.

**Volumen de revisión S:** las 2520 sesiones y el máximo de tres propuestas dan una cota de **7560 propuestas aceptadas**, aunque se esperan menos al detener cada sesión en el primer éxito completo. Con una aceptada por sesión serían 2520. Dos revisiones por aceptada son 5040 revisiones en ese escenario y hasta 15120 en el peor caso; a 2 minutos por revisión equivalen a 168 y 504 horas-revisor, más adjudicaciones. Reservar 25% adicional: 210 horas estimadas, 630 de cota de planificación inicial. El segundo estudio de agentes de §9.5 duplica esos volúmenes: máximo conjunto 22680 aceptadas, 45360 revisiones y 1890 horas-revisor con reserva. Se pueden usar dos revisores agentes locales distintos del implementador, con veredictos trazables y adjudicador independiente; el tiempo/recursos reales se mide en el piloto y no se presupone servicio de pago. La estimación no rebaja el 100%: si se agota la reserva, no se puntúa S ni se congela con aceptadas sin revisar; se registra el bloqueo de evidencia. E tiene su presupuesto separado de doble codificación. El manifiesto debe acreditar capacidad de revisión y ejecución antes de iniciar la medición; la generación barata no implica revisión gratuita.

Para E, la unidad es `(sesión, intento de aparición, regla violada, nodo/lugar causal)`, no el mensaje del parser. Dos diagnósticos se agrupan solo si una misma reparación mínima de esa regla en ese lugar elimina ambos sin tocar otro defecto; guardar esa edición y la recomprobación. Una reparación que exige dos cambios independientes conserva dos episodios. El mismo defecto persistente mantiene ID entre intentos; un defecto corregido que se reintroduce recibe ID nuevo; si la causalidad no puede aislarse, marcar INCONCLUSO. Dos revisores codifican independientemente todas las sesiones, sin ver el juicio del otro ni el total por candidata. Antes de adjudicar se publica acuerdo exacto de recuentos y F1 de pares «mismo episodio» (excluyendo pares de distintas sesiones), desglosados por familia/candidata; umbrales ≥90% y ≥0,90. Si no se alcanzan, E queda INCONCLUSO: revisar el manual con casos del piloto y recodificar el conjunto completo, sin ajustar reglas para reducir E. Un tercero adjudica desacuerdos con evidencia; conservar códigos iniciales y finales. Ninguna puerta E se decide con casos causales sin resolver.

### 9.3. Estudio humano descriptivo de comprensión cruzada

**Responsabilidades separadas:** la prueba de agentes de §9.2 es la puerta CONFIRMATORIA principal para elegir superficie. El estudio humano es DESCRIPTIVO: estima comprensión, esfuerzo y diferencias entre cohortes y detecta problemas graves; no demuestra equivalencia ni exige intervalos dentro de ±10 pp o cocientes [0,80, 1,25]. Se conservan las puertas confirmatorias propias de R F6, con participantes nuevos. Esta separación limita lo que se puede afirmar sobre AC §9.6, criterio 2; no lo da por demostrado por ausencia de diferencias significativas.

**Presupuesto cerrado:** piloto de 9 personas (3 Python, 3 Rust, 3 C#), seguido de 36 personas nuevas por cohorte (108), excluyendo autores y piloto. Una única ampliación de 24 nuevas por cohorte lleva el máximo a 60 por cohorte (180), no a miles. Son seis, y después diez, personas por cada uno de los seis órdenes A/B/C en cada cohorte: múltiplos de seis para balancear exposición. Con 90 minutos de preparación, tres sesiones de 45 minutos y 15 minutos de pausas son 4 horas por persona: 432 horas iniciales, 288 de reserva y 36 del piloto; **756 horas-persona máximas**, más 120 horas de organización/análisis y 80 de entrevistas de alertas. Reclutamiento previsto: 18 participantes por semana durante seis semanas, y hasta cuatro semanas adicionales; entrevistas dentro de esa reserva. Es un presupuesto de planificación, sin afirmar disponibilidad ni autorizar pagos. Antes de medir se documentan acceso a voluntarios, responsables y franjas; reservar este trabajo antes del cierre F1. La precisión elegida corresponde a detectar problemas grandes con decenas de participantes, no a certificar brechas pequeñas.

Ni piloto ni participantes de superficie podrán integrar las muestras confirmatorias de F6, su grupo asistido ni el estudio nuevo de incorporación: ya habrán recibido entrenamiento en las tres sintaxis. Podrán aportar comentarios formativos separados. La experiencia principal se comprueba mediante ejercicio breve; se registra experiencia secundaria. No se sustituyen personas por agentes que simulan cohortes.

Preparación: 30 minutos por candidata con referencias equivalentes y práctica ajena a la evaluación. Cada sesión tiene 15 minutos de comprensión y hasta 15 por modificación; variantes isomorfas de los siete programas, asignadas por cuadrado latino y auditadas en piloto. No se revela el origen de las construcciones. Se mantienen 36 preguntas (tres por cada una de doce reglas) y dos modificaciones por candidata. Reglas: declaraciones/caja, delimitación/terminadores, lectura/copia, modificación/transferencia, selección/retorno, Result, rangos, capturas/invocación, async/cancelación, módulos/visibilidad, efectos y texto/orden. Hay 72 cambios por candidata/cohorte inicialmente (10 u 11 por familia) y 120 al máximo (17 o 18 por familia), con balance rotado. Preguntas sin ejecución; modificaciones con editor, documentación y comprobador, sin asistentes. Cada persona aporta su fracción de 36 aciertos y de dos modificaciones, nunca 38 observaciones independientes.

Se publican medias, distribuciones, denominadores e intervalos descriptivos del 95% entre personas, globales y por regla/cohorte; diferencias entre candidatas conservan el emparejamiento. Para proporciones se usan intervalos t de Welch sobre fracciones individuales, acotando presentación a su dominio; remuestreo por persona (10000 réplicas) como sensibilidad, no como búsqueda del resultado favorable. Los intervalos por regla no tienen cobertura simultánea. Confianza y esfuerzo/legibilidad de 1–5, expectativas sobre `record`/`clone`, caja de campos públicos y sobrecargas de §6.2 se informan por separado. El esqueleto común A/B se separa de los diferenciales; no se atribuye causalidad a una palabra por el agregado.

**Coste contextual de B:** dentro de lectura/copia y modificación/transferencia, incluir pares con `F(move)` / `F(move x)`, `move(x)` / `(move)(x)` y `x: view T` / `as view` / `edit view`, con declaraciones y tipos válidos para cada caso. Predecir lectura, transferencia o invocación, binding disponible después y edición mínima correcta. Contrabalancear tareas homólogas A/C y una variante B con identificadores no contextuales (`dato`/`transformar`); el orden y los nombres se fijan en el piloto. Registrar acierto, segundos por respuesta, consultas a la referencia y confusiones modo/nombre; publicar diferencia pareada e intervalo descriptivo. Así se mide el coste de colisión contextual dentro de B y el coste entre superficies, sin premiar un simple reconocimiento de palabras ni sumar sesiones.

**Precisión y márgenes honestos:** para una diferencia de dos proporciones al 50%, con n personas independientes por cohorte, semiancho normal `h = z × sqrt(2 × 0,25 / n)`. Con el antiguo ajuste de 27 contrastes y dos oportunidades, `z = Φ⁻¹(1 − 0,025/(2×27)) ≈ 3,312`, n=100 da `h = 3,312 × sqrt(0,5/100) = 0,2342`, **23,42 pp**; para una sola proporción sería 16,56 pp. Con n=36 y n=60 da 39,03 y 30,23 pp: exigir ±10 pp sería inviable. En el informe descriptivo puntual del 95%, `z=1,96` da 23,10 y 17,89 pp para diferencias, y 16,33 y 12,65 pp para una sola proporción. Son aproximaciones de planificación en el peor caso de varianza 0,25; los intervalos reales usan varianza entre personas y t, sin promesa de potencia. Los umbrales de alerta siguientes son grandes en relación con esa precisión, no márgenes de equivalencia.

**Tiempo sin hipótesis de dispersión artificial:** se retira la presunción de desviación típica de log-tiempo ≤1. Una mezcla equiprobable de éxitos de log-tiempo 3 y fallos con `log(900)=6,802` tiene desviación ≈1,90 aun antes de otras fuentes de variación. Se usa tiempo restringido penalizado en segundos: `min(900, max(1, segundos))` si es correcto; 900 para incorrecto, abandono o timeout. Publicar media por persona de sus dos cambios, mediana, IQR, proporción de fallos/timeouts y tiempos censurados, sin descartar fallos ni afirmar tiempo latente de resolución. Intervalos por remuestreo de personas y distribuciones acompañan los cocientes descriptivos de medias restringidas. La variable está en [1,900], con desviación máxima 449,5; no se presupone rapidez lognormal ni se usa un cociente temporal como puerta de equivalencia.

**Alertas y regla secuencial registrada:** alerta global si alguna cohorte tiene comprensión <60%, cambios correctos <50%, brecha entre cohortes ≥30 pp en cualquiera de ambas o cociente de tiempos medios restringidos ≥2 acompañado de brecha de fallos/timeouts ≥20 pp. Por regla: acierto <50% o confusión modo/nombre ≥30% activa inspección cualitativa, no 108 puertas confirmatorias. Los antiguos objetivos 80%/5⁄6 se muestran como referencias, no como mínimos humanos para elegir. Si hay alerta, o si un intervalo descriptivo global cruza uno de los umbrales de alerta (resultado inconcluso), ampliar **todas** las candidatas/cohortes de 36 a 60, una sola vez; no ampliar solo a quien favorece la clasificación. Conservar datos de la primera tanda, publicar además la tanda nueva y agregado; estos análisis secuenciales siguen siendo descriptivos, sin afirmaciones confirmatorias derivadas de sus intervalos.

La inspección registra respuestas erróneas y hasta seis entrevistas por alerta/cohorte dentro de la reserva de 80 horas; una entrevista puede cubrir varias alertas, con matriz de cobertura explícita. Si no alcanza para investigar un problema potencialmente grave, registrarlo sin resolver, no declarar ausencia de problemas por falta de entrevista. Un problema grave exige patrón repetido por al menos tres personas independientes, causa localizada en una regla y un ejemplo reproducible que induce una interpretación incorrecta del contrato; dos revisores independientes documentan la causa, no solo la puntuación. Un problema grave sin resolver excluye esa candidata hasta rediseño y nueva validación de la regla. Una alerta estadística aislada o un intervalo amplio no la excluye por sí solo. Al llegar a 60, cerrar el informe con problemas, incertidumbre residual y seguimiento explícito en F6; el coordinador registra esos riesgos al elegir. No se pide otra cohorte ni se afirma equidad probada para permitir la decisión previa a F1.

### 9.4. Ejecución del arnés y formato mínimo de evidencia

Secuencia operativa para el futuro ejecutor:

1. Copiar el corpus congelado a un directorio de resultados independiente; verificar SHA-256 del manifiesto, fuentes y ejecutables. Resolver las tres variantes a las mismas identidades y verificar equivalencia de AST normalizado de cada familia.
2. Ejecutar positivos y negativos del oráculo y comprobar que cada negativo alcanza la regla prevista. Contrastar el comprobador actual contra un snapshot de HEAD con batería del validador (AG, lecciones); no modificar el checkout de otro agente. Registrar versiones, entorno y límites.
3. Construir la lista de sesiones producto cartesiano de candidata, tarea, modelo, semilla; barajar con semilla de orden registrada. Ejecutar cada sesión en proceso/contexto aislado y guardar cada propuesta, diagnóstico, edición y evaluación antes de la siguiente llamada.
4. Recoger sesiones humanas con consentimiento, identificadores anónimos y reloj monotónico; conservar respuestas iniciales y cambios, sin credenciales ni datos privados (R §7.2). Abandono y timeout se registran como fracaso, con indicador de censura y tiempo restringido penalizado de §9.3 para el informe descriptivo; respuestas ausentes cuentan incorrectas, sin eliminar a la persona del denominador.
5. Producir `trials.jsonl` y una tabla de episodios. Cada línea incluye `candidate`, `family`, `task_id`, `rule`, `mode` (generation/repair/human), `model_or_cohort`, `subject_id`, `seed`, `attempt`, `stage_reached`, `correct`, `causal_errors`, `diagnostic_count`, `elapsed_ms`, `input_tokens`, `output_tokens`, `timed_out`, hashes de fuente/oráculo y `review_status`. `correct` no puede ser verdadero si falta una etapa que la tarea exige. Los campos no aplicables son null, no cero inventado.
6. Calcular G1 sobre intento 0; R2 sobre cualquier intento 0–2 correcto de la sesión de reparación; E como episodios nuevos acumulados según §9.2. Para humanos, usar persona como unidad de repetición. Publicar tablas por regla/familia/modelo/cohorte, incluyendo denominadores, fallos y etapas NO EVALUADAS.
7. Agentes: intervalos mediante remuestreo por tarea y semilla dentro de cada modelo, 10000 réplicas con semilla publicada, conservando emparejamiento entre candidatas; intervalos simultáneos de 97,5% por máximo estadístico para sus contrastes registrados. Tiempos de agentes: distribución y tiempo restringido al límite, manteniendo censura; no mediana solo de éxitos. Humanos: intervalos descriptivos y tiempo restringido penalizado exactamente según §9.3; conservar emparejamiento entre candidatas y distinguir las dos tandas. No elegir después el método más favorable.

El manifiesto separa los contrastes humanos globales y los 108 por regla (3 pares × 3 candidatas × 12 reglas), todos descriptivos, de los nueve contrastes confirmatorios de elección de agentes (3 pares de candidatas × G1/R2/E). Los umbrales absolutos de agentes se informan con intervalos: la elegibilidad exige umbral puntual y ausencia de evidencia de incumplimiento. La incertidumbre confirmatoria que puede cambiar la elección activa §9.5; la incertidumbre humana activa únicamente la ampliación acotada y el registro de riesgos de §9.3. No se exige un intervalo humano contenido para elegir.

Este procedimiento no requiere que la sintaxis esté incorporada al producto: los adaptadores de laboratorio pueden vivir fuera del frontend de producción. Sí exige verificarlos antes de evaluar participantes. Los resultados de agentes no sustituyen el estudio humano sin asistentes ni el estudio secundario de 18 personas con asistentes de R §7.7.

Para hacer inequívoca la planificación, este script de biblioteca estándar es parte del protocolo: el ejecutor puede guardarlo fuera del árbol de producción y ejecutarlo con `python planificar.py registro.json`. Lee un registro ya congelado, verifica cobertura y huellas de los archivos y emite las 2520 sesiones JSONL por salida estándar; no llama modelos ni altera fuentes. `registro.json` debe contener `models` (dos IDs/versiones), `order_seed` (entero), `tasks` (84 objetos) y `files` (mapa ruta→SHA-256). Cada tarea contiene `id`, `family` (`EX01`…`EX07`), `mode` (`generation`/`repair`), `prompt_file`, `oracle_file` y `rule`; ambos archivos deben figurar en `files`. Los archivos se resuelven respecto al directorio del registro. El texto de tareas/oráculos y sus revisiones independientes son prerrequisitos, no se autogeneran de las respuestas del modelo.

```python
import hashlib
import itertools
import json
from pathlib import Path
import random
import sys

registry_path = Path(sys.argv[1]).resolve()
root = registry_path.parent
registry = json.loads(registry_path.read_text(encoding="utf-8"))
models = registry["models"]
tasks = registry["tasks"]
assert len(models) == 2 and len(set(models)) == 2
assert len(tasks) == 84 and len({t["id"] for t in tasks}) == 84
assert {t["family"] for t in tasks} == {f"EX{i:02d}" for i in range(1, 8)}
assert {t["mode"] for t in tasks} == {"generation", "repair"}
for family in sorted({t["family"] for t in tasks}):
    for mode, count in (("generation", 4), ("repair", 8)):
        assert sum(t["family"] == family and t["mode"] == mode
                   for t in tasks) == count
for relative, expected in registry["files"].items():
    path = (root / relative).resolve()
    assert path.is_relative_to(root), relative
    assert hashlib.sha256(path.read_bytes()).hexdigest() == expected.lower()
for task in tasks:
    assert task["rule"]
    for key in ("prompt_file", "oracle_file"):
        assert task[key] in registry["files"]
registry_hash = hashlib.sha256(registry_path.read_bytes()).hexdigest()
rng = random.Random(registry["order_seed"])
blocks = list(itertools.product(sorted(tasks, key=lambda t: t["id"]),
                                models, range(5)))
rng.shuffle(blocks)
session = 0
for task, model, seed in blocks:
    candidates = list("ABC")
    rng.shuffle(candidates)
    for candidate in candidates:
        print(json.dumps({"session": session, "candidate": candidate,
                          "task_id": task["id"], "family": task["family"],
                          "mode": task["mode"], "model": model, "seed": seed,
                          "max_proposals": 3, "registry_sha256": registry_hash},
                         ensure_ascii=False, sort_keys=True))
        session += 1
assert session == 2520
```

Requiere Python 3.9 o posterior por `Path.is_relative_to`. En el registro se fija también la versión exacta del intérprete del arnés. Un fallo de aserción detiene la planificación: no se omite una familia ni se sigue con un oracle faltante. Las hojas de referencia específicas A/B/C, los adaptadores y la plantilla común de prompt también deben figurar en `files`; `prompt_file` identifica la tarea semántica común, que el ejecutor combina con la hoja de la candidata asignada, sin cambiar su contrato.

### 9.5. Tabla de puntuación y decisión

Cada criterio recibe 0–5; peso aplicado como `peso × puntuación / 5`, total sobre 100. Se publican valores crudos y razones, no solo el total. **Todas las celdas A/B/C están sin medir**; no se rellenan con preferencias del diseñador.

| Criterio AC §9.6 | Peso | Regla de puntuación antes de medir | A | B | C |
|---|---:|---|---|---|---|
| 1. Mérito propio | 20 | Un punto por cada puerta documentada: justificación funcional de cada familia; parseo sin tipos; informe de legibilidad por cohorte con intervalos y comentarios; cobertura completa del corpus; ninguna ventaja de biblioteca o tarea introducida para una candidata. | NO EVALUADO | NO EVALUADO | NO EVALUADO |
| 2. Equilibrio entre cohortes | 25 | Un punto por evidencia de cada obligación: cohortes/órdenes balanceados y exclusiones; informe global con precisión; desglose por regla y coste contextual; alertas investigadas y ampliación aplicada cuando corresponde; ausencia de problemas graves sin resolver y riesgos residuales registrados. Mide diligencia de la evaluación descriptiva, no equivalencia humana demostrada. No se otorgan puntos por brechas pequeñas imprecisas. | NO EVALUADO | NO EVALUADO | NO EVALUADO |
| 3. Coherencia | 20 | Un punto por cada puerta: mapa exhaustivo superficie→AST; cero ambigüedades sin regla; ida/vuelta e idempotencia verificadas; una salida canónica por árbol y trivia; inventario de símbolos sin sobrecargas semánticas no justificadas. | NO EVALUADO | NO EVALUADO | NO EVALUADO |
| 4. Convención visual | 10 | Un punto por cada puerta: clasificación correcta de todas las categorías; comprobación de estilo; refactorización privada resuelta; bloqueo de colisiones/API observable; conservación de árbol resuelto y efectos tras formato/refactorización. | NO EVALUADO | NO EVALUADO | NO EVALUADO |
| 5. Agentes y medición | 25 | 0 sin evidencia válida; 1 si hay protocolo completo pero falla G1/R2; 2 si ambos alcanzan global pero falla algún estrato; 3 si G1/R2 y E pasan todos los estratos; 4 si además G1≥90% y R2≥95% global; 5 si además E≤0,25/0,10 y ninguna reparación introduce violación de contrato. | NO EVALUADO | NO EVALUADO | NO EVALUADO |

Puertas no compensables: integridad de invariantes, cero aceptaciones semánticas indebidas S, corpus completo para superficie, criterio 2 =5, criterio 4 =5 y criterio 5 ≥3. G1/R2/E constituyen la puerta confirmatoria principal; el estudio humano exige completar el protocolo y resolver problemas graves según §9.3, sin mínimos de equivalencia; una puntuación estética alta no compensa fallos. Las reglas se registran antes de medir. Falta de evidencia no vale cero de rendimiento: impide declarar elegible y se informa NO EVALUADO/INCONCLUSO.

Entre elegibles, seleccionar mayor total si la diferencia es ≥5/100 y las incertidumbres no permiten invertir la conclusión. Si la diferencia es <5 puntos, desempatar solo con coste de cambio **medido simétricamente**: partiendo del adaptador ya construido de cada candidata, aplicar el mismo cambio de un token de modo, un delimitador y una regla de terminación, con fixtures equivalentes y equipos/órdenes contrabalanceados. No comparar «mantener A cuesta cero» con construir B/C. Gana el menor tiempo solo si el intervalo del cociente queda por debajo de 0,90 frente a cada alternativa empatada y no introduce más regresiones; publicar también archivos manuales afectados. Registrar réplicas y análisis del ensayo antes de ejecutarlo. Si solo queda una candidata elegible, se selecciona sin comparación contra candidatas excluidas. El coste histórico de migración desde A se informa como planificación, no como bono de incumbencia. Que I04+ use A no aporta puntos, menor coste ficticio, prioridad, voto ni preferencia en ningún desempate; se aplica exactamente la misma regla a las tres.

Si la incertidumbre confirmatoria cruza la selección o persiste empate, ejecutar una única segunda muestra **de agentes**, independiente, con diez semillas nuevas por modelo/tarea (5040 sesiones; 7560 entre ambos estudios), candidatas y reglas congeladas. No combinar muestras para obtener otra oportunidad sin corrección: cada estudio decide con nivel familiar 97,5%, reparto de α=0,05 entre dos oportunidades para los contrastes registrados. Los intervalos del ensayo simétrico de coste también usan 97,5% simultáneo para sus tres pares. El estudio humano solo amplía de 36 a 60 según §9.3, independientemente del empate de agentes. Si ninguna candidata cumple por defecto demostrado, no repetir para borrar el fallo: devolver la regla a diseño con evidencia y regresión.

**Cierre acotado antes de F1:** registrar el calendario y capacidad de cómputo local del máximo de 7560 sesiones, revisión de §9.2 y diez semanas humanas más dos de análisis antes de empezar; los adaptadores y oráculos son prerrequisitos, no trabajo oculto en ese plazo. Tras las dos oportunidades, seleccionar cuando se cumplan puertas y regla de diferencia/coste. Si persiste el empate, la incertidumbre confirmatoria impide seleccionar o no hay elegibles, la decisión terminal es **rechazar la congelación de este conjunto**, con causa y siguiente cambio de diseño escrito, no dejar un reclutamiento indefinido pendiente ni favorecer A. Las puertas de evidencia no se levantan para cumplir una fecha. El coordinador debe resolver selección o rechazo antes del cierre F1; un rechazo requiere nuevo diseño o resolución expresa del propietario para cerrar la fase. El protocolo hace ejecutable la decisión con recursos acotados, no garantiza que alguna sintaxis merezca seleccionarse. A puede continuar en I04+ sin valor de selección mientras se resuelve el diseño. Ninguna reserva autoriza gasto.

Un híbrido posterior debe especificarse como una **cuarta candidata cerrada**, con tabla de deltas y nueva medición independiente para las reglas cambiadas y sus interacciones. No se toman retrospectivamente las mejores puntuaciones de A/B/C para atribuirlas a un lenguaje jamás evaluado. El coordinador registra selección y motivos por escrito según AG, autoridad delegada; el propietario conserva aprobación de fases y los requisitos de revisión de R F0/AC §7.

## 10. Migración barata y continuación de I04+

### 10.1. Fronteras que deben aislarse

AC §9.6 describe ya separación de lexer/parser/impresor; A2 §§9–10 propone IR resuelta y frontend separado. Este ADR exige mantener y verificar las siguientes fronteras, sin afirmar que estén todas implementadas:

| Componente | Responsabilidad y exclusiones |
|---|---|
| Lexer por versión de superficie | Tokens, trivia, offsets UTF-8 y reglas de saltos; no tipos, propiedad ni selección de implementación. |
| Parser/adaptador de superficie | CST con trivia y errores; AST normalizado, expansiones explícitas y mapa nodo→origen. Prohíbe recuperar aceptando nodos inválidos. |
| AST común | Declaraciones y operaciones semánticas, modos como enum, identificadores resueltos después, rangos con extremo explícito; no necesita saber si un préstamo se escribió `inout`, `edit` o `update`. |
| Impresor canónico | AST→texto de una versión; tabla de grafías, precedencia y paréntesis. No infiere tipos, cambia efectos o renombra APIs. |
| Formateador | CST/trivia y política canónica; compara árboles antes/después. El servicio resuelto de renombrado es separado del impresor (§2.3). |
| Resolución/checker/IR/intérprete | Reciben operaciones normalizadas e identidades; ningún match sobre lexemas de palabras clave para decidir permisos. Origen fuente viaja como metadato, no como instrucción ejecutable. |
| Diagnósticos | Código, subcausa, operación e identidades estables; vocabulario de tokens, edición sugerida y spans pertenecen al adaptador de superficie. |

La IR de A2 §9 conserva procedencias, permisos, transferencias, efectos, limpiezas y origen fuente. I04+ puede continuar con A como superficie de trabajo sin valor de decisión: no espera a la selección entre A/B/C. Nuevos nodos de IR deben justificarse por operaciones, no por grafías. Identidades estables y hashes por función del diseño de IR se registran conforme AC §9.8; no se define aquí el mecanismo de hot reload ni se cambia su alcance. Los hashes semánticos excluyen trivia/grafías, pero incluyen contratos y dependencias pertinentes; el hash de fuente para caché/diagnósticos sigue siendo distinto.

### 10.2. Fixtures en dos capas, con oráculos independientes

1. **Semánticas**: AST tipado/resuelto o constructor declarativo independiente con IDs de nodos, entradas, salida, efectos, permisos y traza de limpieza esperados. El esperado se transcribe del ADR, nunca del ejecutable bajo prueba. Un caso se imprime en A/B/C y se vuelve a analizar; los tres deben producir la misma operación normalizada y el mismo veredicto semántico. Se comparan árboles, no bytes de spans entre superficies.
2. **Textuales**: pequeño corpus irreducible por regla de lexer/parser/impresor/diagnóstico, con fuentes literales, tokens, offsets y `expected_tokens` exactos. Incluye saltos tras return/value:, UTF-8/CRLF, EOF sin salto, genéricos/comparaciones, cadenas/interpolación, comas finales, llamadas de campo y palabras contextuales. Estos esperados **no se generan con el mismo parser/impresor** que se valida.
3. Para trasladar ubicación de una prueba semántica, el impresor emite un mapa independiente de intervalos por NodeId; el test exige señalar el nodo causal. Esto no reemplaza las pruebas de spans reales: la capa textual sigue comprobando los bytes originales y el esquema de A2 §13.6. Los `expected_tokens` de superficie se fijan manualmente por versión, no se borran para abaratar migración.
4. Cada fallo corregido añade regresión; cada negativo rompe una sola regla y su positivo pareado comprueba aceptación. Verificar mutación inversa, resultado esperado y restauración SHA-256; para blobs de base se usa hash del blob git, no del working tree (AG, reglas/lecciones). Fuzz de entrada válida/inválida sin panic, límites explícitos y mismo comportamiento check/run. Nombres de tests describen comportamiento, no rondas superadas.

Propiedades obligatorias: `Normalize(Parse(Print(ast))) = Normalize(ast)`; `Format(Format(text)) = Format(text)`; `Normalize(Parse(Format(text))) = Normalize(Parse(text))`; resolución y efectos iguales antes/después; conservación de comentarios y bytes de literales. Igualdad tras renombrado privado usa un mapa explícito de IDs, no elimina arbitrariamente nombres del contrato público. Los negativos sintácticos no se regeneran desde un AST válido, porque se perdería precisamente el defecto que estudian.

### 10.3. Secuencia de transición y coste que permanece

Mientras se decide, I04+ añade el mínimo de casos textuales exigido por sus reglas y concentra el resto en AST/IR. No se para el trabajo de propiedad/efectos/intérprete; no se rebajan diagnósticos para reducir fixtures. Mantener versión de superficie en manifiesto del arnés y artefactos, sin heurística de «probar tres parsers hasta que uno acepte».

Después de seleccionar: implementar adaptador nuevo aislado; comprobar corpus y regresiones; ensayo de migración en copia; comparar AST resuelto, resultados y eventos; publicar conversor de la versión antigua a la nueva con mapa de ubicaciones; migrar corpus/documentación por lotes trazables. El conversor propone **fuentes**, nunca nuevos esperados textuales: tokens, spans, códigos, subcausas y campos de cada fixture se retranscriben/revisan independientemente contra la regla del ADR. Un negativo no convertible conserva su fuente histórica y recibe un equivalente nuevo validado por separado. La biblioteca necesita mapa explícito de API y plan para consumidores; renombrarla no se disfraza de formateo.

Durante transición se aceptan versiones explícitas en proyectos separados, nunca mezcla de dialectos en un archivo. El conversor antiguo puede seguir como herramienta de importación; el producto congelado emite una sola superficie canónica. Revertir la selección significa cambiar el adaptador y volver a imprimir, manteniendo historial de fuentes/oráculos; no reescribir git ni la base.

«Barato» no significa gratuito: lexer/parser, impresor, diagnóstico textual, documentación, resaltado/LSP, nombres públicos y pruebas de errores seguirán requiriendo trabajo. El ensayo de §9.5 cuantifica horas, archivos manuales y fallos de equivalencia; la meta propuesta es que **cero cambios semánticos en checker/IR/intérprete** sean necesarios para un cambio solo ortográfico o de delimitadores. Si aparece uno, se investiga acoplamiento o cambio semántico real antes de continuar. La inclusividad nueva de B se contabiliza aparte, no se usa para negar esa meta.

### 10.4. Inventario reproducible de deuda textual existente

Censo local sobre el commit **`f808a0c03b05fda4e499d627a747bfcd6fc2c0d3`**, no sobre archivos que otro agente pueda estar editando. Unidad externa: archivo `.nx` en `corpus/`. Unidad embebida: ocurrencia de literal Rust, normal/raw/byte, cuyo contenido contiene una cabecera `fn Nombre(` según el patrón de abajo; incluye fuentes negativas truncadas después de `(`, plantillas y esperados de impresión. No se deduplican: dos copias mantenidas son dos lugares que revisar. No es conteo de tests ni de programas únicos, ni un inventario exhaustivo de fragmentos sin cabecera.

| Ubicación | Archivos `.nx` o sitios de literal con cabecera |
|---|---:|
| `corpus/pass/` | 5 archivos, 1062 bytes de blobs |
| `corpus/fail/` | 10 archivos, 661 bytes de blobs |
| `src/check/names.rs` | 1 sitio |
| `src/check/tests.rs` | 237 sitios |
| `src/diagnostic.rs` | 162 sitios |
| `src/frontend/lex.rs` | 4 sitios |
| `src/frontend/parse/tests.rs` | 613 sitios |
| `tests/canonical_print.rs` | 5 sitios |
| `tests/cli.rs` | 1 sitio |
| `tests/corpus.rs` | 19 sitios |
| `tests/i01_diagnostic.rs` | 152 sitios |
| Total | **15 archivos externos + 1194 sitios embebidos en 9 archivos Rust** |

Comando reproducible desde la raíz en PowerShell; Python y git solo leen blobs locales. Para listar cada sitio, sustituir el incremento por una impresión de ruta y `source.count("\n", 0, match.start()) + 1`, conservando el incremento. El extractor es un censo léxico acotado, no usa el parser de Nexo ni genera esperados.

```powershell
@'
import re
import subprocess
revision = "f808a0c03b05fda4e499d627a747bfcd6fc2c0d3"
def blob(path):
    return subprocess.check_output(["git", "show", revision + ":" + path])
paths = subprocess.check_output(
    ["git", "ls-tree", "-r", "--name-only", revision], text=True
).splitlines()
for prefix in ("corpus/pass/", "corpus/fail/"):
    files = [p for p in paths if p.startswith(prefix) and p.endswith(".nx")]
    print(prefix, len(files), sum(len(blob(p)) for p in files))
literal = re.compile(
    r'//[^\n]*|/\*.*?\*/|(?:br|r)(?P<h>\#{0,255})"(?P<raw>.*?)"(?P=h)'
    r'|(?:b)?"(?P<normal>(?:\\.|[^"\\])*)"', re.S
)
total = 0
for path in paths:
    if not (path.endswith(".rs") and path.startswith(("src/", "tests/"))):
        continue
    source = blob(path).decode("utf-8")
    count = 0
    for match in literal.finditer(source):
        body = match.group("raw")
        if body is None:
            body = match.group("normal")
        if body is not None and re.search(r"\bfn\s+[A-Za-z_][A-Za-z_0-9]*\s*\(", body):
            count += 1
    if count:
        print(path, count)
        total += count
print("sitios_embebidos", total)
'@ | python -
```

Alcance del censo: excluye fragmentos sin esa cabecera, fuentes construidas enteramente por concatenación, variantes generadas, documentación y archivos no versionados; un literal plantilla puede producir muchos casos. El patrón de comentarios no es un lexer completo de Rust (por ejemplo, comentarios de bloque anidados); el inventario de migración deberá auditar esas exclusiones y clasificar cada sitio como fuente, plantilla o esperado. Por tanto, **1194 no es una estimación del total de fixtures independientes** ni una promesa de ediciones automáticas; es carga textual identificable y reproducible, además de los 15 archivos. El ensayo de §9.5 debe registrar por separado horas de conversión de fuentes y de revisión manual de esperados, proporción convertible, archivos tocados y regresiones. Con estos datos no se afirma que la migración ya sea barata: su coste sigue sin medir.

## 11. Riesgos y objeciones previstas

| Riesgo/objeción | Evidencia o control exigido | Consecuencia si persiste |
|---|---|---|
| A parece favorecer conocimientos de Rust/C++ | Cohortes sin autores del compilador, tareas por regla e intervalos descriptivos. La familiaridad no se presume perjudicial ni beneficiosa. | Investigar alertas y excluir A solo ante problema grave sin resolver según §9.3; no inferir equidad de un intervalo amplio. |
| B es un collage y todavía comparte mucho con A | Inventario de símbolos; tabla §5.3; una expansión de if y un impresor; medir errores de modos y palabras contextuales. | Simplificar y volver a medir, no sumar alias. |
| C exige demasiados cierres o texto | Errores de delimitación, tiempo, tokens y comprensión; pruebas de anidación y recuperación local. | Su regularidad formal no compensa un problema humano grave sin resolver ni una puerta de agentes fallida. |
| PascalCase cambia resolución o JSON | Pruebas de colisión, exportación, etiquetas e identidad; campo `estado` intacto; modo check sin renombrado inseguro. | Bloquear cambio automático y registrar deuda de API. |
| Selección B reintroduce retorno implícito | AST esperado con entrega de rama; negativos de closure sin return y de bloque sin value; positivo `let value`. | Retirar abreviatura si no se puede explicar/comprobar de modo local. |
| Sintaxis alternativa altera D11/D12 | Misma matriz de modos/capturas/Call*, precedencia await/?, cancelación y limpieza por nodo. | No tratarlo como cambio cosmético; revisión semántica separada. |
| Prueba «completa» oculta mocks o features diferidas | Etapa alcanzada por tarea; EX01–EX04 literales; fases y contratos de biblioteca separados. | NO EVALUADO en lo no comprobado; no contar aceptación por parser como ejecución. |
| Fixtures generadas aprueban el mismo bug del impresor | Oráculos semánticos independientes y corpus textual manual; mutación inversa y comparación con HEAD. | Invalidar evidencia dependiente, conservar el defecto como regresión. |
| Tamaño humano o dispersión temporal impiden precisión útil | n=36→60 por cohorte, precisión descriptiva y tiempo restringido de §9.3; dos oportunidades de agentes en §9.5. | Cerrar incertidumbre como riesgo, sin afirmar equivalencia; seleccionar o rechazar congelación con presupuesto acotado, no ampliar indefinidamente. |
| Un modelo conoce más una sintaxis | Dos modelos fijados, contexto nuevo, material simétrico, resultados por modelo/familia. | Limitar conclusión a herramientas probadas; no atribuir causalidad universal. |
| Mantener tres frontends encarece I04+ | Adaptadores de laboratorio separados; A provisional; IR sin lexemas y superficie versionada. | Reducir alcance del experimento, sin arrastrar tres dialectos al producto. |
| Publicar sintaxis antes de validar biblioteca | Interfaces congeladas para medir, sin promesa de ejecución F5 ni aprobación de fase. | Conservar puertas pendientes y no declarar lenguaje terminado. |

## 12. Alternativas descartadas y recomendación revisable

### 12.1. Alternativas descartadas en esta propuesta

- Adoptar íntegra EXT por ser del propietario: AC §9.6 la declara candidata. Introduciría `fails`, opcionales con `?`, tuberías, propiedades, defer y otros cambios semánticos sin resolver sus interacciones.
- Prohibir toda influencia reconocible o repartir una cuota de palabras por lenguaje: contradice la aclaración de AC §9.6 y no mide lectura ni reparación.
- Mantener simultáneamente `consume/move/own`, `fn/function`, lógica simbólica/verbal y varias formas de captura en producción: multiplica decisiones y fixtures sin utilidad independiente. Aquí son alternativas de laboratorio mutuamente excluyentes.
- Adoptar indentación significativa como cuarta candidata en esta comparación: requiere fijar ancho/tabulaciones, mezcla de espacios, continuación multilínea, dedent en fuente incompleta y recuperación de ediciones pegadas; C no representa ni permite inferir esos comportamientos. El alcance actual fija tres adaptadores y 24 bloques: añadirla exige ocho bloques más, otro adaptador, contrabalancear los órdenes de exposición de cuatro candidatas (24 permutaciones) y ampliar el presupuesto humano y de agentes. Se difiere por ese coste concreto y por gramática aún sin especificar, no por supuesta inferioridad. La conclusión quedará restringida a A/B/C; si ninguna cumple o si las señales de delimitación de la cohorte Python justifican explorarla, el siguiente diseño deberá incluir una candidata de indentación completa antes de registrarla. No se usará el resultado de C como evidencia contra ella.
- Renombrar símbolos con regex, por caja o mediante lexer: no resuelve referencias, colisiones ni API observable. Abarata una edición rompiendo semántica.
- Añadir una superficie exclusiva para agentes: sin evidencia de necesidad; duplica documentación y contradice el objetivo de coherencia. El AST común sirve como interfaz de herramientas, no como segundo lenguaje de usuarios.
- Cambiar todos los fixtures a texto generado y eliminar spans: haría invisible una clase de errores del frontend y violaría evidencia requerida por AG/A2 §13.
- Congelar ahora o aplazar la elección hasta después de F1: la primera opción omite medición; la segunda aumenta precisamente la deuda que AC §9.6 identifica.

### 12.2. Recomendación

Se recomienda **construir, verificar y congelar los tres adaptadores a la vez**, sin recomendar una candidata ganadora ni priorizar B para prototipar. La selección corresponde a la medición de §9, conforme AC §9.6, criterios 1–2. Nombres uniformes y ausencia de tuberías, `fails`, `prop` y `defer` son comunes a A/B/C; lógica verbal es común a B/C. No se contabilizan como ventajas exclusivas de B.

Los diferenciales a medir son explícitos: A conserva lectura sin marca de firma y abreviatura de closure; B exige modo en parámetros, usa `move` uniforme, rango semiabierto `..<` y selección `if then`; C cambia declaraciones/bloques a cabeceras verbales y `end`, tipos a `of[...]` y terminación a `;` obligatorio. A/B comparten esqueleto de declaraciones y bloques, lo que exige el desglose por cohortes de §9.3; C no se presume neutral por alejarse de ese esqueleto. La tabla §6.2 registra costes comunes y diferenciales sin atribuir superioridad no medida.

**Condición de decisión:** aplicar elegibilidad, comparación y regla neutral de empate/inconcluso de §9.5; no mover umbrales ni atribuir a un híbrido resultados ajenos. A solo puede continuar como superficie de trabajo de I04+, sin valor de selección. Antes de cerrar F1 debe existir selección documentada o resolución explícita del propietario sobre la puerta pendiente; este ADR no concede una excepción tácita al calendario.

### 12.3. Evidencia pendiente para la revisión

El validador debe contrastar cada requisito como PASA / FALLA / NO EVALUADO / INCONCLUSO con evidencia propia, según AG. Esta entrega no asigna esos veredictos. Pendientes: revisión independiente del diseño; gramáticas/adaptadores de laboratorio; corpus adversarial y mutaciones; verificación semántica de ejemplos contra interfaces; formateador y migración de nombres; piloto, registro confirmatorio y resultados; decisión previa al cierre F1. Permanecen además las puertas originales, incluidos dos revisores sin discrepancias en F0 y aprobación de fases por el propietario (R F0; AC §7; AG, reglas comunes). Ninguna demostración ni cantidad de tests sustituye esas obligaciones.

## Iteraciones de Flujo A

- 2026-10-20, iteración 1: propuesta de tres superficies, corpus común, medición y migración; revisión independiente señaló nueve defectos y objeciones adicionales.
- 2026-10-20, iteración 2: excepciones de nombres y cobertura completa de conversión; recomendación neutral; inventario A/B/C de sobrecargas; `move` uniforme y parámetro propietario en EX06; revisión del 100% para S y procedimiento de acuerdo para E; tamaño y precisión global fijados con informe por regla descriptivo; empate sin incumbencia; límites documentados entonces de EX05 y del arnés async de EX06; deuda textual cuantificada, indentación diferida con límites explícitos y separación de participantes respecto de F6. La revisión independiente de esa iteración señaló dos defectos pendientes: cortocircuito no observable y presupuesto humano incompatible con la decisión previa a F1. Sin mediciones de candidatas ni aprobación de fase.

- 2026-10-20, iteración 3: EX05 añade efecto contable y comprobaciones de ambas rutas y de sus modos A/B/C; revisión humana descriptiva de 36→60 personas por cohorte con presupuesto, precisión explícita, alertas y ampliación única; agentes como puerta confirmatoria principal y decisión terminal previa a F1. Se retira la hipótesis de dispersión logarítmica y se usa tiempo restringido penalizado. Se ejercita conversión histórica sobre las ocurrencias del corpus, se mide `move`/`view` contextual, se cuantifica revisión completa S y se explicita que I04+ no da ventaja a A. Se conservan los límites del arnés async de EX06 y las puertas de fase. Pendiente de revisión independiente; sin mediciones de candidatas ni aprobación.

Comprobación documental local de esta iteración: el script de conversión recorre 24 bloques, 1743 ocurrencias y 168 aplicaciones de filas históricas; detecta omisiones simuladas de `read_text`, `max_connections` y `SoA` sin modificar el archivo. Se recalcularon los semianchos y se revisaron las firmas/llamadas y las cuatro comprobaciones de EX05 por candidata. Esto no acredita ejecución Nexo ni medición humana o de agentes; los adaptadores siguen pendientes. Archivo conservado en UTF-8 sin BOM.
