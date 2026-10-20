# F0-0001 — Primer corte exploratorio de valores y recursos

Estado: implementado como experimento; revisión independiente PASS limitada al corte.
Evidencia e incidencias: ../validation/F0-0001-review.md.
No modifica la base 0.2, no aprueba F0 ni decide D11–D13.

## Motivo y alcance

Contrastar primero P02 y la limpieza de D07 con un frontend real, comprobador
estático y ejecución separada. Arrays anidados usan copias físicas profundas;
los recursos no implementan Clone. Evita fingir CoW, préstamos o un runtime completo.

Rust 1.99.1, edición 2024, sin dependencias externas. Un crate con módulos syntax,
check y runtime, biblioteca y CLI. La representación ejecutable es el AST ya
comprobado encapsulado en CheckedProgram: **todavía no hay IR propia bajada**.

## Gramática del subconjunto (provisional)

program = "fn main()" block EOF
block = "{" statement* "}"
statement = ("let" | "var") name "=" expression ";"
          | name ("[" expression "]")* "=" expression ";"
          | "assert_eq(" expression "," expression ");"
          | "print(" expression ");"
          | ("drop(" | "inspect(") name ");"
          | "return;" | "fail();" | block
expression = postfix ("+" postfix)*
postfix = atom ("[" expression "]")*
atom = i64_literal | "true" | "false" | name | "(" expression ")"
     | "[" expression ("," expression)* "]"
     | "move" name | "mock_open(" expression ")"

Los nombres son ASCII y el texto fuente UTF-8; comentarios // pueden contener
Unicode. Se requiere punto y coma en este corte: la inserción opcional D01 aún NO
está implementada. Todo el archivo debe consumirse; no se ignoran características
no soportadas. Una sola función main, sin parámetros ni resultado explícito.

## Tipos y acceso

Tipos inferidos: i64, bool, Array<T> homogéneo y MockResource. Bool no es entero.
Arrays de distinta longitud pueden tener el mismo tipo. Arrays vacíos se rechazan
porque faltan anotaciones de elemento; no se inventa un tipo comodín.

let es inmutable; var permite asignación del mismo tipo y mutación indexada.
Copia lógica de arrays significa copia física recursiva en este prototipo.
Los recursos solo pueden crearse o transferirse como inicializador completo.
Copia implícita, recurso anidado, reasignación de recursos y uso después de move/drop
se rechazan estáticamente. Sombreado permitido en bloque hijo, duplicación prohibida
en el mismo bloque. Un inicializador ve el ámbito anterior al nuevo binding.

inspect es una primitiva de lectura instantánea de pruebas, no un sistema general
de préstamos. No conserva vistas, no ofrece inout ni acredita exclusividad inferida.
No existen closures, tareas, referencias compartidas ni capturas en este corte.

## Evaluación y limpieza

Expresiones de datos se evalúan izquierda→derecha. + es comprobado para i64 en debug
y release. Negativos solo como literales; no se soporta resta, división ni flotantes.
Una asignación evalúa índices de izquierda a derecha, luego RHS, comprueba límites
y escribe. No hay efectos externos en expresiones de datos de este subconjunto.

Cada adquisición mock_open obtiene identidad única y etiqueta i64. No abre archivos,
sockets ni handles del sistema. Eventos open/inspect/close son evidencia simulada.
Limpieza en orden inverso de declaración del ámbito propietario, al salir, retornar
o propagar fallo recuperable. Move cambia el propietario, no crea otra adquisición.
Drop cierra explícitamente y deja el slot consumido. Si el inicializador falla, no
se registra propietario ni adquisición inexistente. Ámbitos externos se limpian
al propagarse un fallo. El runtime no promete limpiar tras pánico del anfitrión,
OOM, cierre del proceso o abort; no confundirlos con fail() recuperable simulado.

R0101/R0102/R0201/R0301 son errores recuperables del intérprete de pruebas, **no una
implementación de Result, ?, try_add o de la política completa de panic Nexo**.
Deben separarse al ampliar el lenguaje; T07/T08 solo reciben cobertura parcial.

## Comprobación y efectos

check analiza el archivo completo, incluso declaraciones tras return. Es una regla
conservadora del corte y puede rechazar código inalcanzable; no se afirma análisis de
flujo completo. run usa exactamente el mismo frontend y checker antes de ejecutar.
No hay inferencia de efectos ni contratos no_alloc/no_io/no_suspend: esas sintaxis
se rechazan. No etiquetar el AST como verificado respecto de efectos no implementados.

## Límites operativos y confianza

Fuente máxima 1 MiB (la CLI limita también la lectura). Recursión de parser, altura
compuesta de expresiones y profundidad de tipos de array inferidos tienen límites
independientes de 128. El límite de altura compuesta se comprueba al construir cada
nodo, antes de recorrer o destruir un árbol profundo. Envolver variables sucesivas
no elude el límite de tipos.

Ejecución con un millón de unidades: una por sentencia, una por expresión de datos
y una por celda copiada desde un binding. Indexar consume su temporal ya propietario
sin una segunda copia oculta. La limpieza necesaria no se interrumpe por agotamiento
del presupuesto. Esta contabilidad no incluye cada operación del anfitrión, búsqueda
de nombres, formato, igualdad, recorrido de conteo ni destrucción; no es una cota de
tiempo o de bytes del proceso. No es una garantía
no_alloc ni una medición de coste del futuro código nativo. La memoria usada por
instrumentación y el compilador no representa memoria del lenguaje de producción.
Base de confianza: parser, checker, runtime, Rust/std y primitives mock declaradas.

## Evidencia y pendientes

Corpus pass/fail y tests de integración verifican códigos concretos, salidas y orden
exacto de eventos. No contabilizar un fallo de parser como validación de propiedad.
Revisión adversarial del corte completada; no equivale a auditoría externa.
PENDIENTE: IR, efectos, préstamos, traits, estados D09, Result,
generadores, EX01/EX04, D11–D13, plataformas y todos los protocolos F0.
