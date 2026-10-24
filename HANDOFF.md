# Continuación de Nexo

Estado vigente (2026-10-24): la sintaxis oficial B usa el recorrido AST de `frontend::print` y la identidad del token fuente se resuelve por span exacto o límite estructural. Los comentarios junto a paréntesis omitidos tienen regresiones LF, CR y CRLF. La biblioteca enumera 767/769 pruebas en debug/release (736/738 activas y 31 ignoradas), más 80 integraciones por perfil. La campaña de comentarios consecutivos comprobó 30.000 fuentes debug y 300.000 release, con dos a cuatro comentarios por hueco y hasta tres huecos, sin fallos. La validación separada, el respaldo/publicación y las puertas de F0 siguen pendientes. I07c e I08–I11 siguen pendientes.
Las 31 campañas ignoradas terminaron 31/31 en release tanto en paralelo (714,63 s) como con un hilo (1.128,05 s). Incluyen el fuzz estructurado de 400.000 fuentes y cuatro semillas para lugares indexados, además de una muestra debug de 10.000 fuentes. `edit` aislado se rechaza durante el análisis sintáctico con NX-E0002/`inout_place` antes de construir la IR (`adr_gap_edit_outside_admitted_context`); F0-D15 §7.1 no asigna a ese caso un `pending_scope`.
`sh scripts/validate.sh` terminó en cero desde un `target` limpio y en una copia LF con el mismo `target`; 92 archivos se inspeccionaron, 18 se normalizaron, ninguno conservó CR y la copia fue eliminada.
F0-CODIGOS está sincronizado con F0-D15 §§3.6/6.5: incluye `implementation_limit`, `pending_scope` y los 30 `feature_id`. La discrepancia de primario D03 de F0-D15 §8.12 sigue asignada a I10b; véanse las notas de implementación.
Las llamadas de usuario e intrínsecas comparten la comprobación secuencial de argumentos en la frontera de propiedad. `AssertEq` conserva el préstamo del primer lugar durante el segundo argumento: escritura, índice y `edit` anidado reciben NX-E1105 en check y run. La campaña release de operandos secuenciales cubrió 400.000 fuentes, cuatro semillas y 54 formas: 377.677 aceptadas por el parser, 800.000 llamadas de check/compilación y 370.000 ejecuciones. La batería de lugares indexados cubrió otras 400.000 fuentes, 800.000 llamadas y 399.920 ejecuciones, con transferencia, destrucción, escritura y préstamo dentro del índice. Ambas terminaron sin `InvalidIr`, `internal_error` ni pánicos en esos dominios. La igualdad de arrays seguida de escritura, transferencia o `edit` de ambos operandos tiene regresiones positivas. La laguna normativa específica de intrínsecas se registra como `adr_gap_intrinsic_sequential_arguments`.

El formato reubica comentarios de tokens omitidos por clase y vecinos en el orden de emisión; F0-D15 §5.2 fija la regla completa y exige anclas monótonas por orden de fuente. Los casos de paréntesis omitidos, orden tras `;` y grupos reparsan, conservan AST normalizado y comentarios, e idempotizan en los dominios medidos. La campaña dirigida previa cubrió 892 posiciones aceptadas y 30.000/200.000 fuentes comentadas en debug/release, sin fallos en ese generador. La validación separada y las demás puertas de F0 siguen pendientes. La asignación a nombre de función devuelve NX-E0006/`function_value` en check y run; su clasificación como place sigue fijada por `adr_gap_function_name_is_not_an_assignable_place`.

I07b fue validada tras validación separada y publicada. La sustitución por B recibió una validación separada con defectos abiertos; no tiene publicación.
El propietario decide el inicio de cada fase.

## Leer primero

1. README.md y Pending.md: estado real, ejecución y pendientes.
2. docs/baseline/Nexo_ROADMAP_rev0.2.md y Nexo_CAMBIOS_rev0.2.md completos.
3. docs/F0_ACUERDOS.md §9.6 y docs/adr/F0-D15-sintaxis-oficial.md: superficie oficial y migración.
4. docs/adr/F0-0002-second-slice.md y docs/adr/F0-CODIGOS.md: contratos del pipeline.
5. docs/validation/F0-D15-implementation-notes.md: implementación y límites.

La base 0.2 permanece intacta. No crear una revisión 0.3 ni dar por aprobada F0.
El implementador y el validador deben ser distintos; el validador no implementa ni
rebaja los requisitos para aprobar. Cada avance terminado debe respaldarse en
GitHub y verificarse contra el estado remoto, sin force push ni secretos.

## Qué hay implementado

Un crate Rust 1.99.1 sin dependencias: frontend oficial B (lexer, parser, CST,
impresor y formateador), checker de nombres/tipos/propiedad, IR verificada,
intérprete y CLI `check`/`run` sobre una sola vía. Arrays anidados se copian
físicamente. Los recursos mock solo se transfieren y cierran; no representan
handles reales. Hay 5 archivos positivos, 10 negativos por código esperado y
76 pruebas de integración enumeradas por perfil (9 CLI, 27 corpus, 3 de impresión
canónica y 37 de diagnósticos).

Hay arneses reproducibles y un modelo Python de comparación diferencial.
La sustitución B requiere validación separada; consultar sus notas para el alcance.

## Cómo verificar tras clonar

Instalar o usar Rust 1.99.1 con rustfmt y clippy. Python 3 se usa solo por los arneses
independientes. Desde la raíz:

```sh
sh scripts/validate.sh
cargo build --locked --offline
cargo build --locked --offline --release
```

No confiar en binarios de un workspace anterior. El script principal ejecuta fmt,
clippy estricto y tests debug/release. Las pruebas no requieren red. Los arneses
publicados de F0-0001 documentan su propio commit y no son puertas operativas de B.

## Tres defectos encontrados y corregidos (registro del corte publicado)

- Altura compuesta del AST: combinar cadenas y paréntesis eludía los límites
  locales y causaba stack overflow en debug. Se limita altura al construir nodos.
- Índices negativos de asignación: se comprobaban antes del RHS, contradiciendo
  el orden declarado. Se evalúan todos los índices y el RHS antes de los límites.
- Lectura CLI truncada: cortar un emoji en el límite confundía tamaño con UTF-8
  inválido. Se limita lectura de bytes, comprueba tamaño y después decodifica.

Además se acotó profundidad inferida de arrays entre bindings, se protegió el
renderizado de diagnósticos UTF-8 y se probó limpieza al agotar presupuesto.

## Próximo trabajo concreto

1. Someter las correcciones de la sintaxis oficial B de F0-D15 a validación separada; después I07c (tipos diferidos de `var` unificados) e I08–I11 según §17 del ADR F0-0002.
2. Mantener una sola vía de frontend, comprobación, IR y ejecución en `nexo check`/`nexo run`; la retirada del prototipo ocurrió en la sustitución B.
3. Extender los tres flujos reducidos exigidos por F0: herramienta, actualización
   de datos y petición simulada. EX01/EX04 completos no están implementados.
4. Completar los modelos y protocolos de F0, incluidos estados D09, plataformas,
   IR/backend, invariantes, H-* y T01–T28. No reducir F0 a una lista de demos.

En B, `move` es contextual para transferencia y `edit` para préstamo exclusivo; los terminadores opcionales forman parte del frontend oficial. Result, efectos comprobados, closures y generadores conservan sus límites nominales conforme a F0-D15 §7.1 y las etapas posteriores.

## I01 (registro del corte publicado)

I01 implementa el esquema de diagnósticos, su presentación JSON/texto, `nexo explain` y lectura acotada; fue validada tras validación separada y publicada en `d4836cf`. I02a fue validada y publicada en `f073275`; I02b está validada tras validación separada y publicada en `84d5f53`; I03 validada tras validación separada y publicada en `f808a0c`; I04a validada tras validación separada y publicada en `0739795`; I04b validada tras validación separada y publicada en `f5016d7`; I05 validada tras validación separada y publicada en `a203e51`; I06 validada tras validación separada y publicada en `0eaf876`; I07a validada tras validación separada y publicada en `9f96780`; I07b validada tras validación separada y publicada; I08–I11 pendientes. El addendum de códigos quedó fusionado en F0-CODIGOS.md y eliminado.md. D11–D13 y F0-0002 tienen PASS de diseño, no aprobación de fase. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores. Las lagunas conocidas de I01 constan en docs/validation/F0-0002-I01-implementation-notes.md. F0 sigue pendiente de dos revisores sin discrepancias, T01–T28, plataformas y las demás puertas de Pending.md.

## I02a (registro del corte publicado)

I02a implementa un `src/frontend/lex.rs` separado con tokens y spans de bytes para el perfil léxico F0-0002, usando la API de diagnósticos estructurados de I01 para NX-E0001. No sustituye el lexer actual de `syntax.rs`; `check` y `run` conservan su ruta anterior. LX01–LX05 tienen negativo y positivo pareado con `+`. El alcance, la reproducción y las ambigüedades del ADR constan en `docs/validation/F0-0002-I02a-implementation-notes.md`. I02a está validada tras validación separada y publicada en `f073275`.

## I02b (registro del corte publicado)

I02b está validada tras validación separada y publicada en `84d5f53`. `frontend::parse` y `frontend::print` modelan sintaxis, diagnósticos y texto canónico; `compile`, `check` y `run` siguen la ruta anterior. Las regresiones incluyen `OperationList` vacío ante tokens erróneos, reparación tras trivia, prefijos de sentencias tras saltos y entradas anidadas desde pilas pequeñas. El parser usa un hilo ancho reutilizable y devuelve un diagnóstico si no puede crearlo. La cobertura, los límites y la reproducción constan en `docs/validation/F0-0002-I02b-implementation-notes.md`.

## I03 (registro del corte publicado)

I03 está validada tras validación separada y publicada. `la API de comprobación semántica` usa el AST de I02b, registra firmas y resuelve nombres por ámbito antes de asociar argumentos y comprobar tipos básicos y retornos escritos. Devuelve diagnósticos ordenados de sitios independientes y ejecuta las pasadas semánticas en un hilo acotado de 16 MiB. `src/check/names.rs` y `src/check/types.rs` siguen aislados de `compile`, `nexo check` y `nexo run`. Las notas de alcance, diagnósticos, pruebas y límites están en `docs/validation/F0-0002-I03-implementation-notes.md`. El cierre completo de las fixtures con ejecución, préstamos, control o Result sigue en los pasos que asigna F0-0002 §17.2. I04a validada tras validación separada y publicada en `0739795`; I04b validada tras validación separada y publicada en `f5016d7`; I05 validada tras validación separada y publicada en `a203e51`; I06 validada tras validación separada y publicada en `0eaf876`; I07a validada tras validación separada y publicada en `9f96780`; I07b validada tras validación separada y publicada; I08–I11 pendientes; F0 conserva sus puertas pendientes.

La prueba de escalado de I03 usa 16 familias por defecto y 24 con `NEXO_FULL_SCALE`, de 256 KiB a 1 MiB. El tiempo de la suite completa y las razones medidas se consignan en las notas de I03; la prueba de escalado preexistente del parser domina la duración. Los límites y las lagunas observadas están en esas notas.

## I04a (registro del corte publicado)

I04a validada tras validación separada y publicada. `ir::compile_scalar` baja el AST aceptado de I02b, tras `la API de comprobación semántica`, a una IR escalar tipada y verificada. `FunctionId` es el nombre estable; el índice de tabla se ordena por nombre. Cada cuerpo tiene un hash FNV-1a de una codificación canónica sin spans. El lowering usa un hilo de 16 MiB y el verificador comprueba ámbitos, orígenes únicos y límites de spans. `interp::run_scalar` obtiene `main` de la tabla de funciones verificada y registra la salida en `interp/observation.rs`. `compile`, `nexo check` y `nexo run` conservan su ruta publicada.

Las pruebas cubren `integer_limits`, literal fuera de rango, basura final, R0103, L04, el positivo escalar de L07, bindings, reasignación completa y las reparaciones escalares de §14. El verificador tiene pruebas de referencias, tipos, consumo de temporales, metadatos, efectos y limpieza. La batería generada compara 8.000 programas con un modelo decimal independiente; 20.000 fuentes mutadas validan diagnósticos. La medición separada de 15 familias de 256 KiB a 1 MiB dio razones entre 3,754 y 4,211. `cargo test --locked --offline --lib -- --list` enumera 257 pruebas; una medición de escalado queda marcada para ejecución explícita.

Los límites y la discrepancia de offsets entre comentarios antiguos del corpus, CRLF del árbol de trabajo y la tabla migrada del ADR constan en `docs/validation/F0-0002-I04a-implementation-notes.md`. En I04a, arrays, recursos, llamadas Nexo, control por CFG, Result, efectos, cuotas completas y CLI ampliada permanecían en los pasos I04b–I11. Ningún resultado del corte aprueba el corte ni F0.

## I04b (registro del corte publicado)

I04b validada tras validación separada y publicada en `f5016d7`. La IR representa tipos y literales de array, copias físicas profundas, lectura indexada prestada en contexto de lectura y copia del elemento en contexto propietario, longitud y escritura indexada sobre una raíz local mutable. La destrucción explícita del propietario libera el binding antes de crear la unidad. Los NX-E0105 de sitios independientes se conservan y ordenan por span. El verificador rechaza índices y elementos mal tipados, raíces ausentes o inmutables, valores prestados instalados como propietarios y préstamos vivos durante escritura o destrucción. El hash canónico incluye las nuevas operaciones, raíces, índices, operadores y literales; la tabla sustituible conserva `FunctionId` por nombre. El intérprete cobra celdas por cabeceras y elementos y comprueba el RHS antes de los límites de una escritura indexada.

`docs/validation/F0-0002-I04b-implementation-notes.md` registra las pruebas, filas cerradas y límites. `compile`, `nexo check` y `nexo run` conservan su ruta anterior. I05–I11 mantienen recursos, llamadas de usuario, préstamos generales, control, Result, efectos, cuotas completas e integración. F0 sigue pendiente de dos revisores sin discrepancias y aprobación del propietario.

## I05 (registro del corte publicado)

I05 validada tras validación separada y publicada. La IR verificada admite parámetros ordinarios de lectura y propietarios, `CallDirect` con asociación en orden escrito, transferencia explícita, recursos mock y retorno explícito. El intérprete ejecuta frames acotados, adquiere, inspecciona y cierra recursos en `interp/observation.rs`, prepara resultados antes de limpiar los locales del callee y libera los temporales del llamador después de la llamada. Los arrays prestados a parámetros ordinarios comparten almacenamiento durante la lectura; las copias propietarias siguen siendo profundas. La destrucción explícita consume bindings completos o temporales; la destrucción parcial se rechaza en arguments.

El hash de una función incluye el nombre y la firma completa de cada callee referenciado, incluidos nombres de parámetros, modos, tipos y resultado; no incluye el cuerpo de otra función. La tabla sustituible comprueba de nuevo el programa y restaura el estado anterior si una sustitución falla. Las pruebas incluyen regresiones de propiedad y limpieza, un modelo independiente de 8.000 programas, 20.000 fuentes hostiles, sondas de pila, 27 mediciones de escala y mutaciones restauradas por SHA-256. Las cifras y los límites están en `docs/validation/F0-0002-I05-implementation-notes.md`.

La corrección de I05 usa ids `binding:<offset>` y `parameter:<offset>` en diagnósticos, limpia temporales en orden inverso antes de locales cuando falla una llamada, y conserva el diagnóstico original cuando falla el inicializador de un `let` con usos posteriores. El lexer se comparte entre funciones y los nombres de parámetros se buscan desde su offset. El código inalcanzable conserva diagnósticos sin copiar todo el estado del lowering por bloque; doce familias adicionales cubren retornos anidados y declaraciones densas. La lectura indexada de una raíz consumida o destruida devuelve NX-E0202 con el sitio causal. Estas correcciones fueron revisadas de forma independiente antes de la publicación de I05.

Los comandos `nexo check` y `nexo run` conservan la ruta entonces vigente. I05 fue validada y publicada en `a203e51`; las puertas de F0 permanecen pendientes. I06 añade préstamos exclusivos; I07a cubre control acíclico y bucles con rechazo nominal del punto fijo pendiente; I07b conserva la propiedad sobre retroaristas, e I08–I11 conservan `Result`, efectos, cuotas completas e integración.

## I06 (registro del corte publicado)

I06 validada tras validación separada y publicada en `0eaf876`. `BeginLoan` y `EndLoan` hacen explícitos los permisos de argumentos de llamada; el verificador comprueba procedencia, exclusividad, padre de représtamo, spans, modos y liberación inversa. El intérprete devuelve a su lugar los parámetros de edición exclusiva después de la llamada y también tras un fallo, sin perder el propietario del llamador. El préstamo de una proyección mueve temporalmente el elemento y no añade celdas. Las proyecciones de array se comprueban al prestar; una asignación indexada vuelve a comprobar la raíz después del RHS, que puede llamar a una función con parámetro de edición exclusiva y modificarla.

S06, S08, S30, P05–P07 y PP02 tienen pruebas; P07 usa el primario completo `[73,77)` fijado en el ADR. P05–P07 fijan las cinco claves de `data` y las identidades publicadas. El modelo independiente ejecuta 8.000 programas válidos; dos baterías hostiles recorren 30.000 y 20.000 fuentes. Doce familias densas entre 256 KiB y 1 MiB tienen medición explícita y contador determinista. Revisar `docs/validation/F0-0002-I06-implementation-notes.md` para cifras, reglas y límites. `compile`, `nexo check` y `nexo run` siguen la ruta entonces vigente. F0 aún requiere dos revisores sin discrepancias y aprobación del propietario.


## Corte I07a (registro del corte publicado)

El lowering activa el análisis sin emisión tras una divergencia y en las colas inalcanzables de bloques. Recorre operandos, argumentos, índices y ramas posteriores; los temporales emitidos antes del corte se limpian en el retorno. Los repros de asignación posterior a un binding divergente compilan. Los errores de valor de rama ausente, propiedad tras divergencia y precedencia estructural conservan sus diagnósticos. Las pruebas nuevas comprueban spans y campos exactos en posiciones representativas y la escala de diez familias divergentes y vivas; la matriz exhaustiva combinatoria sigue pendiente, por lo que este trabajo permanece incompleto.

I07a validada tras validación separada y publicada en `9f96780`. El API aislado `ir::compile_scalar` produce CFG para `if`, `else if`, `value:` y `while`; el verificador comprueba saltos, tipos y disponibilidad en las uniones, y `interp::run_scalar` ejecuta bucles con cuotas y limpieza por iteración. NX-E1317 considera las continuaciones normales. La comprobación del código inalcanzable conserva los diagnósticos estructurales y de propiedad de los casos cubiertos, incluidos errores independientes de la misma función; todavía requiere la matriz completa y oráculos externos. La fusión acíclica de propiedad cubre P08; cuando una retroarista exige punto fijo se devuelve `OutsideScalarSlice("ownership fixpoint")`. El corte incorpora tipo interno `Never`, restauración de ámbitos en error, cierre de temporales divergentes, dominadores calculados una vez y lectura escalar independiente en igualdad. El verificador rechaza ciclos de CFG forjados y limita sus recorridos. Consultar `docs/validation/F0-0002-I07a-implementation-notes.md` para filas, pruebas y límites. I07b conserva P09 y la resolución de propiedad en retroaristas. F0 sigue pendiente de dos revisores sin discrepancias y aprobación del propietario.

El trabajo de I07a corrigió los repros de binarios en código muerto, condiciones ramificadas de `while`, búsqueda cuadrática del preheader, tipado de bindings anotados y operadores divergentes, lectura hipotética de bindings diferidos, precedencia estructural de `value:`, declaraciones hipotéticas y ticks de préstamos. Tres familias de bucles medidas de 256 KiB a 1 MiB mantuvieron pasos, bytes y asignaciones con razón máxima 4,335; el reloj quedó por debajo de 5,2. La mutación que restablece el barrido cuadrático falló una regresión y se restauró por SHA-256. Los listados de biblioteca contienen 580 pruebas en debug y 582 en release; la validación ejecutó 569 y 571 pruebas activas con 11 ignoradas por perfil. `validate.sh` terminó en 0 y el comparador archivado en ese commit en cero diferencias (medición del commit citado). El fuzz dirigido de 100.000 fuentes con oráculo propio terminó sin discrepancias; 42 mutaciones nuevas y las supervivientes nominales fueron detectadas y restauradas. Faltan negativos aislados con campos diagnósticos exactos para combinaciones no muestreadas; la evidencia registrada del corte sigue INCOMPLETA. I07a validada tras validación separada y publicada en `9f96780`.


El resultado de expresion distingue `Flow::Value { temp: Option<TempId>, ty }` de `Flow::Diverged { ty: Option<Ty> }`. Literales, nombres, indices, operadores, llamadas, arrays, `if` y bloques de valor usan ese resultado en el recorrido vivo y analizado. `emit` devuelve un temporal opcional. Quedan nueve apariciones de `usize::MAX`: siete son destinos CFG aun sin enlazar, una es el retorno ficticio de `finish_block` durante el analisis y una pertenece a un assert de test. Ninguna decide la divergencia de expresiones. Los usos de `== Ty::Never` comprueban el tipo interno inferior de bindings sin anotacion cuyo valor inicial diverge; no son centinelas de flujo. El tipo de los operadores procede de su operador y operandos presentes, con independencia del lado divergente. Un `()` derecho de suma o resta numerica da NX-E0102/type_mismatch con tipo esperado entero y tipo encontrado unidad. La decision `adr_gap_never_bottom` trata `Never` como fondo del reticulo: en igualdad y arrays no impone el tipo entero; el primer operando concreto fija el tipo y el diagnostico recae sobre el que lo contradice. La prioridad de NX-E1302/missing_branch_value se aplica a la funcion en el mismo recorrido vivo y analizado.

La regla vigilada de §13.9 conserva cada sitio independiente con izquierdo divergente: el error numérico del operando derecho se emite en el checker y se ordena por span junto con los demás errores de la función. La comprobación estructural de `value:` conserva prioridad cuando falta un valor de rama. El oráculo comparó 300.000 pares de 67 plantillas, incluidas llamadas, arrays, binarios y hasta cuatro errores por función, con código, subcausa, etapa, operación, `data` y spans mapeados: cero diferencias en esa muestra, cero `Never` en `data` y ningún fallo interno. La muestra no acredita cobertura exhaustiva. La medición previa de 76 familias registró razones máximas de 4,14 en bytes y asignaciones; el nuevo recorrido de errores requiere una medición independiente de escala.

`cargo test --locked --offline --lib -- --list` enumera 607 pruebas en debug y 609 en release, con 12 ignoradas por perfil; las activas son 595 y 597. La batería `ir::` contiene 349 activas y 12 ignoradas; `check::` contiene 98 activas. Las veinte mutaciones nuevas de las reglas de operandos y `Never` fueron detectadas en debug y release con restauración SHA-256. I07a validada tras validación separada y publicada en `9f96780`. Siguen abiertas la matriz combinatoria exhaustiva, la validación separada y las puertas de fase del propietario. `sh scripts/validate.sh` terminó en 0 y el comparador archivado en ese commit devolvió cero diferencias (medición del commit citado).

Los límites `adr_gap_deferred_var_alias` y `OutsideScalarSlice("lowering depth")` constan en las notas de I07a. I07a validada tras validación separada y publicada en `9f96780`.

## Corte I07b (registro del corte publicado)

I07b validada tras validación separada y publicada. El análisis de propiedad resuelve las retroaristas de `while` mediante un trie persistente de hechos dispersos. La cabecera converge antes de propagar la salida del bucle. La condición completa entrega un único estado de propiedad a las salidas verdadera y falsa, con independencia de los literales de los brazos de `if`. P09 conserva `maybe_consumed`, el testigo `flow.backedge` y el sitio `value.drop`. Las escrituras completas reinicializan el binding y los locales del cuerpo nacen de nuevo en cada iteración.

La cuota `ownership_flow_steps` es por función. Al alcanzar NX-E0003 se conservan solo los diagnósticos obtenidos de bloques estables y de bucles convergidos; se elimina cualquier observación intermedia del punto fijo y se deja una sola entrada por código, span primario y operación. La subcausa de un uso unido prevalece sobre la de un estado anterior. Se ordenan por span y se analizan las funciones posteriores. La CLI publicada conserva `nexo check` y `nexo run`. La regla conservadora de comparaciones y los demás límites nominales están en `docs/validation/F0-0002-I07b-implementation-notes.md`. I07a está publicada en `9f96780`; el corpus independiente de terminaciones está en `45886fd` y CI en `f6f94b9`/`97257cd`. I08–I11, los dos revisores sin discrepancias y la aprobación de fase del propietario siguen pendientes.

El listado de biblioteca contiene 673 pruebas en debug y 675 en release, con 19 ignoradas por perfil; `ir::` ejecuta 408 activas en debug y 408 en release. Las pruebas de cuota comparan diagnósticos estables con la ejecución sin límite, también durante la pasada diagnóstica, y las uniones de nodos iguales fijan el testigo de menor distancia y la definitividad condicional. `sh scripts/validate.sh` terminó en 0 en el árbol principal y en la copia LF; el comparador archivado en ese commit devolvió cero diferencias (medición del commit citado).

## Sintaxis oficial B

El frontend B reside en `src/frontend/`; las pruebas del parser tienen correspondencia individual. La suite actual contiene 767 unitarias en debug (736 activas, 31 ignoradas), 769 en release (738 activas, 31 ignoradas) y 80 integraciones por perfil. Las campañas de asignación y trabajo comparten una exclusión mutua para que sus contadores no interfieran al ejecutarse juntas. La corrección de modos repetidos reconoce el segundo prefijo antes del descenso recursivo y conserva la clasificación de un nombre contextual aislado.

Los 15 archivos de corpus, sus pruebas, la CLI y los comandos `check`/`run` ya usan B. `src/syntax.rs` y `src/runtime.rs` se retiraron. La siguiente validación separada debe contrastar las correcciones con casos y criterios propios antes de I07c; la fase y las puertas F0 permanecen sin aprobación. La sustitución aún no tiene commit ni publicación remota.

El formato con comentarios utiliza el recorrido AST del impresor y asocia cada comentario a la identidad CST de su token anterior, siguiente o EOF. Cuando un literal entero con signo funde signo y dígitos, solo el signo conserva ancla exacta; el comentario de los dígitos se reubica como el de un token omitido. Los dos repros tras `return` y `value:` y doce contextos adicionales reparsan, conservan AST, comentarios e idempotencia en LF, CR y CRLF. Se comparó la ejecución original y formateada de 100.000 fuentes aceptadas con comentarios en debug y release, sin diferencias de resultado u observaciones. La matriz de omisiones y comentarios consecutivos comprueba reparseo, AST normalizado, texto, orden e idempotencia en sus dominios medidos; los resultados y límites constan en las notas F0-D15.

La extensión oficial del código fuente es `.n`: los 15 archivos del corpus se renombraron sin alterar bytes. La CLI procesa también rutas sin extensión; `tests/cli.rs` compara `check` y `run` para ambas rutas, y `tests/corpus.rs` exige los 15 nombres oficiales y rechaza archivos residuales de la extensión anterior. La revisión independiente y las puertas de F0 siguen pendientes.
