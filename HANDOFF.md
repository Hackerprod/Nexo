# Continuación de Nexo

Estado local I08: Result<T,E>, Ok/Err, ? y TryAdd recorren checker, IR verificada e intérprete; run aplica la puerta única `names::entry` después de lowering, conservando el `Resolved` de `check_ast`. `check_proposed(run_entry)` mantiene su orden de prueba. Las regresiones k4/k5, prioridad check/run y E0202 I07b están en `tests/i08.rs`; la fuente de cuota B usa 32768 `Ok(1)` expandidos. Las deudas A, I07b, I10a/I10b y la aprobación arquitectónica de la enmienda ADR están en Pending.md. Las reglas, pruebas y límites figuran en docs/validation/F0-0002-I08-implementation-notes.md; la evidencia medida se genera en docs/validation/F0-0002-I08-estado.md. La revisión independiente, publicación, I09-I11 y las puertas de F0 permanecen pendientes.

Estado local I07c (2026-10-31): la inferencia monomórfica conserva procedencias y estados internos separados; el lowering consume `InferredTypes`. F0-0002 §4.1.2 fija los diagnósticos de restricciones diferidas: los tipos de constructores pueden estar determinados con componentes pendientes, cada constructor Array exige resolver su elemento al cierre, las uniones conservan testigos parciales y la contaminación suprime solo consecuencias causales. El arnés RD transcribe todas las fuentes expandidas del ADR con huella del fragmento y compara el nivel normativo de `check`/`run`; las diferencias de instantánea se informan. Las cifras de pruebas, campañas, fuzz y mutaciones se regeneran en `docs/validation/F0-0002-I07c-estado.md`. I07c está local, sin publicación ni aprobación de fase.
La población cerrada de producción se enumera con `scripts/i07c_mutaciones.py` respecto de `b506d7d` y queda clasificada en `docs/validation/i07c-mutations/`. Las cifras medidas se regeneran en `docs/validation/F0-0002-I07c-estado.md`. La deuda de cobertura de código sin cambios está en `Pending.md`; las reglas están en las notas de I07c.
Los repros diferidos de aritmética, comparación, igualdad y join tienen regresiones concretas. El emisor `record_error` reclama cada sitio una vez; la reserva duplicada del cierre diferido se eliminó. `scripts/i07c_gemelos.py` mide como instrumento informativo los gemelos comparables y clasifica las diferencias permitidas por §4.1.2.7 con pares RD. Las familias de aceptación frente a `b506d7d` están en `head_acceptance.json` y tienen prueba pareada.
Los pares de agrupación terminaron sin spans de grupo, diagnósticos duplicados, fallos internos ni desacuerdos entre check y run. Los repros CLI de arrays y programas profundos conservaron check/run coherentes. El estado generado registra las campañas ignoradas.
Los tres bloqueantes posteriores de tipos tienen regresiones de programa: los conflictos de arrays emiten tipos completos del operando, los sitios numéricos independientes sobreviven a un elemento pendiente y `<` fija el operando diferido por orden de fuente aun con un hermano contaminado o una base pendiente. `scripts/i07c_span_audit.py` clasifica los usos directos de `.span()`; el corpus normativo, el diferencial y el fuzz incluyen estas familias. El estado generado contiene las cifras verificadas.
El plan de formas conserva la procedencia de la primera igualdad, incluida su marca intrínseca, durante asignaciones completas compatibles y dentro de bloques. Una comparación con constructor derecho irresuelto registra el dominio independiente del operando izquierdo junto a su igualdad pendiente y lo decide al cierre; una igualdad cruzada resuelta conserva prioridad. Las regresiones de programa también fijan el tipo conocido de `Len`, la conservación del binding tras asignación rechazada y los testigos parciales de uniones exteriores (§§4.1.2.2–5, 13.15).
Una condición fallida de unión anidada conserva el NX-E0103 independiente de un `[]` usado en otra rama (§§4.1.2.2 y 4.1.2.5). La regresión de programa fija el primario y los campos exactos; las adjudicaciones causales de mutación citan cada regla en `docs/validation/i07c-mutations/causal-adjudication.json`.

La sintaxis oficial B conserva un solo recorrido AST para impresión y formato; la identidad de tokens se resuelve por span y límites estructurales. Los comentarios y terminadores insertados tienen regresiones LF, CR y CRLF. `edit` aislado se rechaza con NX-E0002/`inout_place`, y `AssertEq` mantiene préstamos durante la evaluación de sus argumentos. La obligación de emisión del primario D03 de F0-D15, sección 8.12, corresponde a I10b. I08 espera revisión independiente; I09-I11 y las puertas de F0 siguen pendientes.
F0-CODIGOS está sincronizado con F0-D15 §§3.6/6.5: incluye `implementation_limit`, `pending_scope` y los 30 `feature_id`. La discrepancia de primario D03 de F0-D15 §8.12 sigue asignada a I10b; véanse las notas de implementación.
Las llamadas de usuario e intrínsecas comparten la comprobación secuencial de argumentos en la frontera de propiedad. `AssertEq` conserva el préstamo del primer lugar durante el segundo argumento: escritura, índice y `edit` anidado reciben NX-E1105 en check y run. Las campañas de operandos secuenciales y lugares indexados terminaron sin `InvalidIr`, `internal_error` ni pánicos en sus dominios; la evidencia de esas campañas está en las notas del corte correspondiente. La igualdad de arrays seguida de escritura, transferencia o `edit` de ambos operandos tiene regresiones positivas. La laguna normativa de intrínsecas queda descrita en las notas del corte.

El formato reubica comentarios de tokens omitidos por clase y vecinos en el orden de emisión; F0-D15 §5.2 fija la regla completa y exige anclas monótonas por orden de fuente. Los casos de paréntesis omitidos, orden tras `;` y grupos reparsan, conservan AST normalizado y comentarios, e idempotizan en los dominios medidos. La evidencia del corte correspondiente está en sus notas. Las puertas de F0 siguen pendientes. La asignación a nombre de función devuelve NX-E0006/`function_value` en check y run; su clasificación como place sigue fijada por `adr_gap_function_name_is_not_an_assignable_place`.

I07b fue validada tras validación separada y publicada. La sustitución por B está publicada en `a56740e`; las observaciones de su registro pertenecen a ese corte.
El propietario decide el inicio de cada fase.

## Leer primero

1. README.md y Pending.md: estado real, ejecución y pendientes.
2. docs/baseline/Nexo_ROADMAP_rev0.2.md y Nexo_CAMBIOS_rev0.2.md completos.
3. docs/F0_ACUERDOS.md §9.6 y docs/adr/F0-D15-sintaxis-oficial.md: superficie oficial y migración.
4. docs/adr/F0-0002-second-slice.md y docs/adr/F0-CODIGOS.md: contratos del pipeline.
5. docs/validation/F0-D15-implementation-notes.md: implementación y límites.

La base 0.2 permanece intacta. No crear una revisión 0.3 ni dar por aprobada F0.
El implementador y el revisor independiente deben ser distintos; el revisor independiente no implementa ni
rebaja los requisitos para aprobar. Cada avance terminado debe respaldarse en
GitHub y verificarse contra el estado remoto, sin force push ni secretos.

## Qué hay implementado

Un crate Rust 1.99.1 sin dependencias: frontend oficial B (lexer, parser, CST,
impresor y formateador), checker de nombres/tipos/propiedad, IR verificada,
intérprete y CLI `check`/`run` sobre una sola vía. Arrays anidados se copian
físicamente. Los recursos mock solo se transfieren y cierran; no representan
handles reales. Hay archivos positivos y negativos por código esperado. El listado
vigente de biblioteca e integración está en `docs/validation/F0-0002-I07c-estado.md`.

Hay arneses reproducibles y un modelo Python de comparación diferencial. La sustitución B está publicada en `a56740e`; sus notas fijan el alcance.

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

1. Cerrar I07c (tipos diferidos de `var` unificados) y continuar con I08–I11 según §17 del ADR F0-0002.
2. Mantener una sola vía de frontend, comprobación, IR y ejecución en `nexo check`/`nexo run`; la retirada del prototipo ocurrió en la sustitución B.
3. Extender los tres flujos reducidos exigidos por F0: herramienta, actualización
   de datos y petición simulada. EX01/EX04 completos no están implementados.
4. Completar los modelos y protocolos de F0, incluidos estados D09, plataformas,
   IR/backend, invariantes, H-* y T01–T28. No reducir F0 a una lista de demos.

En B, `move` es contextual para transferencia y `edit` para préstamo exclusivo; los terminadores opcionales forman parte del frontend oficial. Result, efectos comprobados, closures y generadores conservan sus límites nominales conforme a F0-D15 §7.1 y las etapas posteriores.

## I01 (registro del corte publicado)

I01 implementa el esquema de diagnósticos, su presentación JSON/texto, `nexo explain` y lectura acotada; fue validada tras validación separada y publicada en `d4836cf`. I02a fue validada y publicada en `f073275`; I02b está validada tras validación separada y publicada en `84d5f53`; I03 validada tras validación separada y publicada en `f808a0c`; I04a validada tras validación separada y publicada en `0739795`; I04b validada tras validación separada y publicada en `f5016d7`; I05 validada tras validación separada y publicada en `a203e51`; I06 validada tras validación separada y publicada en `0eaf876`; I07a validada tras validación separada y publicada en `9f96780`; I07b validada tras validación separada y publicada; I08–I11 pendientes. El addendum de códigos quedó fusionado en F0-CODIGOS.md y eliminado.md. D11–D13 y F0-0002 tienen veredicto favorable de diseño, no aprobación de fase. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores. Las lagunas conocidas de I01 constan en docs/validation/F0-0002-I01-implementation-notes.md. F0 sigue pendiente de dos revisores sin discrepancias, T01–T28, plataformas y las demás puertas de Pending.md.

## I02a (registro del corte publicado)

I02a implementa un `src/frontend/lex.rs` separado con tokens y spans de bytes para el perfil léxico F0-0002, usando la API de diagnósticos estructurados de I01 para NX-E0001. No sustituye el lexer actual del prototipo inicial; `check` y `run` conservan su ruta anterior. LX01–LX05 tienen negativo y positivo pareado con `+`. El alcance, la reproducción y las ambigüedades del ADR constan en `docs/validation/F0-0002-I02a-implementation-notes.md`. I02a está validada tras validación separada y publicada en `f073275`.

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

I07a validada tras validación separada y publicada en `9f96780`. El API aislado `ir::compile_scalar` produce CFG para `if`, `else if`, `value:` y `while`; el verificador comprueba saltos, tipos y disponibilidad en las uniones, y `interp::run_scalar` ejecuta bucles con cuotas y limpieza por iteración. NX-E1317 considera las continuaciones normales. La comprobación del código inalcanzable conserva los diagnósticos estructurales y de propiedad de los casos cubiertos, incluidos errores independientes de la misma función; todavía requiere la matriz completa y modelos independientes. La fusión acíclica de propiedad cubre P08; cuando una retroarista exige punto fijo se devuelve `OutsideScalarSlice("ownership fixpoint")`. El corte incorpora tipo interno `Never`, restauración de ámbitos en error, cierre de temporales divergentes, dominadores calculados una vez y lectura escalar independiente en igualdad. El verificador rechaza ciclos de CFG forjados y limita sus recorridos. Consultar `docs/validation/F0-0002-I07a-implementation-notes.md` para filas, pruebas y límites. I07b conserva P09 y la resolución de propiedad en retroaristas. F0 sigue pendiente de dos revisores sin discrepancias y aprobación del propietario.

El trabajo de I07a corrigió los repros de binarios en código muerto, condiciones ramificadas de `while`, búsqueda cuadrática del preheader, tipado de bindings anotados y operadores divergentes, lectura hipotética de bindings diferidos, precedencia estructural de `value:`, declaraciones hipotéticas y ticks de préstamos. Tres familias de bucles medidas de 256 KiB a 1 MiB mantuvieron pasos, bytes y asignaciones con razón máxima 4,335; el reloj quedó por debajo de 5,2. La mutación que restablece el barrido cuadrático falló una regresión y se restauró por SHA-256. Los listados de biblioteca contienen 580 pruebas en debug y 582 en release; la validación ejecutó 569 y 571 pruebas activas con 11 ignoradas por perfil. `validate.sh` terminó en 0 y el comparador archivado en ese commit en cero diferencias (medición del commit citado). El fuzz dirigido de 100.000 fuentes con oráculo propio terminó sin discrepancias; 42 mutaciones nuevas y las supervivientes nominales fueron detectadas y restauradas. Faltan negativos aislados con campos diagnósticos exactos para combinaciones no muestreadas; la evidencia registrada del corte sigue INCOMPLETA. I07a validada tras validación separada y publicada en `9f96780`.


El resultado de expresión distingue `Flow::Value { temp: Option<TempId>, ty }` de `Flow::Diverged { ty: Option<Ty> }`. Literales, nombres, índices, operadores, llamadas, arrays, `if` y bloques de valor usan ese resultado en el recorrido vivo y analizado. `emit` devuelve un temporal opcional. Quedan nueve apariciones de `usize::MAX`: siete son destinos CFG aun sin enlazar, una es el retorno ficticio de `finish_block` durante el análisis y una pertenece a un assert de test. Ninguna decide la divergencia de expresiones. Los usos de `== Ty::Never` comprueban el tipo interno inferior de bindings sin anotación cuyo valor inicial diverge; no son centinelas de flujo. El tipo de los operadores procede de su operador y operandos presentes, con independencia del lado divergente. Un `()` derecho de suma o resta numérica da NX-E0102/type_mismatch con tipo esperado entero y tipo encontrado unidad. El comportamiento de `Never` como fondo del retículo implica que, en igualdad y arrays no impone el tipo entero; el primer operando concreto fija el tipo y el diagnóstico recae sobre el que lo contradice. La prioridad de NX-E1302/missing_branch_value se aplica a la función en el mismo recorrido vivo y analizado.

La regla vigilada de §13.9 conserva cada sitio independiente con izquierdo divergente: el error numérico del operando derecho se emite en el checker y se ordena por span junto con los demás errores de la función. La comprobación estructural de `value:` conserva prioridad cuando falta un valor de rama. El oráculo comparó 300.000 pares de 67 plantillas, incluidas llamadas, arrays, binarios y hasta cuatro errores por función, con código, subcausa, etapa, operación, `data` y spans mapeados: cero diferencias en esa muestra, cero `Never` en `data` y ningún fallo interno. La muestra no acredita cobertura exhaustiva. La medición previa de 76 familias registró razones máximas de 4,14 en bytes y asignaciones; el nuevo recorrido de errores requiere una medición independiente de escala.

`cargo test --locked --offline --lib -- --list` enumera 607 pruebas en debug y 609 en release, con 12 ignoradas por perfil; las activas son 595 y 597. La batería `ir::` contiene 349 activas y 12 ignoradas; `check::` contiene 98 activas. Las veinte mutaciones nuevas de las reglas de operandos y `Never` fueron detectadas en debug y release con restauración SHA-256. I07a validada tras validación separada y publicada en `9f96780`. Siguen abiertas la matriz combinatoria exhaustiva, la validación separada y las puertas de fase del propietario. `sh scripts/validate.sh` terminó en 0 y el comparador archivado en ese commit devolvió cero diferencias (medición del commit citado).

El antiguo límite de alias diferido y `OutsideScalarSlice("lowering depth")` constan en las notas de I07a. I07a validada tras validación separada y publicada en `9f96780`.

## Corte I07b (registro del corte publicado)

I07b validada tras validación separada y publicada. El análisis de propiedad resuelve las retroaristas de `while` mediante un trie persistente de hechos dispersos. La cabecera converge antes de propagar la salida del bucle. La condición completa entrega un único estado de propiedad a las salidas verdadera y falsa, con independencia de los literales de los brazos de `if`. P09 conserva `maybe_consumed`, el testigo `flow.backedge` y el sitio `value.drop`. Las escrituras completas reinicializan el binding y los locales del cuerpo nacen de nuevo en cada iteración.

La cuota `ownership_flow_steps` es por función. Al alcanzar NX-E0003 se conservan solo los diagnósticos obtenidos de bloques estables y de bucles convergidos; se elimina cualquier observación intermedia del punto fijo y se deja una sola entrada por código, span primario y operación. La subcausa de un uso unido prevalece sobre la de un estado anterior. Se ordenan por span y se analizan las funciones posteriores. La CLI publicada conserva `nexo check` y `nexo run`. La regla conservadora de comparaciones y los demás límites nominales están en `docs/validation/F0-0002-I07b-implementation-notes.md`. I07a está publicada en `9f96780`; el corpus independiente de terminaciones está en `45886fd` y CI en `f6f94b9`/`97257cd`. I08–I11, los dos revisores sin discrepancias y la aprobación de fase del propietario siguen pendientes.

El listado de biblioteca contiene 673 pruebas en debug y 675 en release, con 19 ignoradas por perfil; `ir::` ejecuta 408 activas en debug y 408 en release. Las pruebas de cuota comparan diagnósticos estables con la ejecución sin límite, también durante la pasada diagnóstica, y las uniones de nodos iguales fijan el testigo de menor distancia y la definitividad condicional. `sh scripts/validate.sh` terminó en 0 en el árbol principal y en la copia con finales LF; el comparador archivado en ese commit devolvió cero diferencias (medición del commit citado).

## Sintaxis oficial B: registro del corte publicado en `a56740e`

El frontend B reside en `src/frontend/`; las pruebas del parser tienen correspondencia individual. La suite de ese corte contenía 767 unitarias en debug (736 activas, 31 ignoradas), 769 en release (738 activas, 31 ignoradas) y 80 integraciones por perfil. Las campañas de asignación y trabajo comparten una exclusión mutua para que sus contadores no interfieran al ejecutarse juntas. La corrección de modos repetidos reconoce el segundo prefijo antes del descenso recursivo y conserva la clasificación de un nombre contextual aislado.

Los 15 archivos de corpus, sus pruebas, la CLI y los comandos `check`/`run` ya usan B. Los archivos del prototipo inicial se retiraron. La sustitución está publicada en `a56740e`. La fase y las puertas F0 permanecen sin aprobación.

El formato con comentarios utiliza el recorrido AST del impresor y asocia cada comentario a la identidad CST de su token anterior, siguiente o EOF. Cuando un literal entero con signo funde signo y dígitos, solo el signo conserva ancla exacta; el comentario de los dígitos se reubica como el de un token omitido. Los dos repros tras `return` y `value:` y doce contextos adicionales reparsan, conservan AST, comentarios e idempotencia en LF, CR y CRLF. Se comparó la ejecución original y formateada de 100.000 fuentes aceptadas con comentarios en debug y release, sin diferencias de resultado u observaciones. La matriz de omisiones y comentarios consecutivos comprueba reparseo, AST normalizado, texto, orden e idempotencia en sus dominios medidos; los resultados y límites constan en las notas F0-D15.

La extensión oficial del código fuente es `.n`: los 15 archivos del corpus se renombraron sin alterar bytes. La CLI procesa también rutas sin extensión; `tests/cli.rs` compara `check` y `run` para ambas rutas, y `tests/corpus.rs` exige los 15 nombres oficiales y rechaza archivos residuales de la extensión anterior. Las puertas de F0 siguen pendientes.

## I07c: estado local del corte de corrección

El checker planifica las formas pendientes sin emisiones ni cargos y comprueba después el cuerpo una vez. Las expresiones con dominio pendiente aportan su tipo nominal y una guarda causal al mismo comprobador; la guarda se resuelve al cierre antes de emitir el conflicto de un consumidor y se retira si el dominio se refuta. Las asignaciones de bloques anidados contribuyen al plan de formas. Cada constructor irresuelto conserva NX-E0103 propio, y los cuerpos hipotéticos de `if`/`while` mantienen sus errores independientes aun cuando falle la condición. Dos fallos efectivos de operandos contaminan el resultado de su operación antes de la asignación o condición exterior. Las regresiones de programa y sus gemelos concretos cubren estos casos; la evidencia y sus límites están en `docs/validation/F0-0002-I07c-implementation-notes.md` y en el estado generado de I07c. El oráculo RHS forma parte de la batería ignorada registrada en `i07c-ignored-results.json`.

La población, el diferencial completo de diagnósticos y ejecución, las campañas ignoradas y sus cifras se consultan en el estado generado de I07c. El manifiesto de gemelos tipados/libres cita las excepciones de prefijo incompatible; la clasificación de supervivientes se deriva del diferencial completo con las distinciones N/I del ADR. Esta evidencia es local; la revisión independiente, publicación y puertas de F0 siguen pendientes.
I09 (estado local): solo los parámetros `move` siembran propiedad para la destrucción de salida; `view` y `edit` conservan préstamo (§§4.3, 6.4 puntos 3 y 5, 8.1). El destructor de un temporal prestado a una función de usuario cuenta en el llamador (§§8.1–8.2, 13.15), y `value.drop` de `Drop(...)` abarca la llamada completa sin terminador (§§8.2, 13.15). El código posterior a retorno incondicional se comprueba sin contaminar caminos alcanzables (§6.2). Las pruebas y mediciones locales constan en `docs/validation/F0-0002-I09-implementation-notes.md` y el estado generado; la revisión independiente y aprobación de fase por el propietario siguen pendientes.

## I10a: recuperación del corte

La implementación conserva el rastreador vivo de lowering de §11.8.B, elimina el cobro abreviado de ownership y compara tracker y analizador disperso en funciones sin bucle. Los límites de ownership diferencian scope function y compilation; los perfiles reducidos son internos. El límite `ir_items` se reporta después de propiedad y efectos, y bloquea también el análisis de clausuras que dependan de una función no construida. Los índices por BlockId, liveness persistente y programación dinámica de testigos reducen los términos de escala registrados en los informes de diseño.

Reproducir con `cargo test --locked --offline`, `cargo test --locked --offline --release`, `cargo clippy --locked --offline --all-targets -- -D warnings` y `sh scripts/validate.sh` en una copia limpia sin archivos temporales. Revisar `docs/validation/F0-0002-I10a-implementation-notes.md`, el estado y resultados de mutación y las notas de pendientes. La campaña tipada registró diferencias aún no adjudicadas del comparador de I07c; consultar el estado generado. La validación independiente y la decisión de fase del propietario siguen abiertas.

Estado local de verificación I10a: población de 91 reescrituras únicas y 91 resultados; 82 detectadas, siete supervivientes y dos no compilables. Biblioteca debug 1054 aprobadas/52 ignoradas y release 1056/53; RD 152/152, fmt, clippy -D warnings y validate.sh en clon limpio concluyeron con código 0. La campaña de razón de tiempo (release, ignorada) usa umbral 12: el control lineal queda por debajo y las reposiciones cuadráticas de los escaneos por salida lo superan; una reposición marginal del escaneo de bloques por salida queda en el borde y se registra como riesgo residual de rendimiento, cuyas unidades sí se cuentan en los contadores de trabajo.

## I10b: corrección local tras la validación independiente

Val43 reportó B1–B4. La corrección local conserva 2/6/1, 56/25/15, `Print=10`, `AssertEq(1<2,true)=19`, 152 pares RD y el selector fallido DG01–DG03. Suites debug/release, clippy, fmt y `sh scripts/validate.sh` en clon limpio concluyeron; las campañas registran 100.000 fuentes debug y 300.000 release con semillas distintas. La población real contiene 90 mutaciones: 85 detectadas y cinco supervivientes equivalentes; los quince no equivalentes murieron y todas las fuentes restauraron SHA-256. Consultar `docs/validation/F0-0002-I10b-implementation-notes.md` y `docs/validation/F0-0002-I10b-state.json` para evidencia y límites. La revisión independiente y la aprobación de fase del propietario siguen pendientes.

Estado local I11a (2026-11-07): la ruta única incorpora el entorno mock explícito, intrínsecas verificadas, transacciones de observación y corrección de índices propietarios/prestados. Consultar `docs/validation/F0-0002-I11a-implementation-notes.md` y el estado generado. La revisión independiente, la aprobación del propietario e I11b siguen pendientes; no declarar cerrada una fase.

Actualización I11a (2026-11-07): validación local completa en debug (1090/53) y release (1092/54), `clippy -D warnings`, fmt y clon limpio con `sh scripts/validate.sh`; RD 152/152 sin diferencias. Campañas nuevas 100.000/300.000 con semillas 117212/117211, reproducidas byte a byte. La mutación real registró 40 defectos detectados y seis equivalencias justificadas (`view_verify_usage`, `env_dup_keys`, `indexvalue_fault_plus1`, `write_index_always_dec`, `lower_write_ty`, `check_type_limit`); cero supervivientes no equivalentes y SHA-256 restaurado en todos los casos. Los JSON de campañas, mutaciones y estado se regeneraron; el clon y su target se eliminaron. Falta revisión independiente y decisión de fase del propietario.

Actualización de cobertura I11a (2026-11-07): las doce mutaciones supervivientes se convirtieron en regresiones; `env_dup_keys` y los dos sitios únicos de `view_verify_usage` se ejecutan ahora como mutaciones. La población completa registró 55 detectadas, cuatro equivalencias restantes y cero supervivientes no equivalentes, con SHA-256 restaurado. Campañas uniformes y mixtas: debug 100.000 (117212, 117213) y release 300.000 (117211, 117214). `sh scripts/validate.sh` en copia limpia sin archivos temporales: debug 1457/63, release 1459/64; RD 152/152, cero diferencias. Revisión independiente y aprobación de fase siguen pendientes.

## I11a — actualización local (2026-11-08)

Se incorporó `tests/mock_oracle.rs` con parser y modelo de runtime independientes y un único adaptador al producto. Campañas reproducibles: debug 100.000 casos/semilla 202611080801 y release 300.000/202611080802, 64 categorías de parser y lotes runtime bajo cells-64; ambos incluyen el testigo de trabajo R0001 en `observed=1000001`. La población completa en copia desechable registra 142 mutaciones detectadas y seis equivalencias justificadas; cero supervivientes no equivalentes y SHA-256 restaurado en todos los casos. `sh scripts/validate.sh` en clon limpio, sin artefactos previos: debug 1459/64, release 1461/65, Clippy `-D warnings`, fmt y RD 152/152 sin diferencias; clon y target retirados. La evidencia está en `docs/validation/F0-0002-I11a-implementation-notes.md`, `F0-0002-I11a-campanas.json`, `F0-0002-I11a-mutaciones.json` y `F0-0002-I11a-estado.json`. No se modificó producción. Revisión independiente, decisión de fase del propietario e I11b permanecen pendientes.

## I11a val55 — continuación local (2026-11-08)

D1 está implementada: se exige lectura completa y JSON sintácticamente válido antes de clasificar duplicados o esquema. Los cinco casos val55 tienen regresiones CLI, y `tests/mock_oracle.rs` comprueba acuerdo entre parser recursivo, parser por tokens/pila y producto, extremos de las cuatro primitivas, registro indexado, JSON diagnóstico y bytes. Campañas finales: 202611080803/debug/100000 y 202611080804/release/300000, 128 categorías y 100/300 lotes indexados/mixtos. Mutaciones finales: 167 registros, 151 detectadas, 15 equivalencias, una deuda D2 y cero supervivientes no equivalentes; regeneración de campañas/mutaciones byte a byte.

`sh scripts/validate.sh` en clon limpio sin temporales ni `target` terminó con debug 1469/65 y release 1471/66; RD 152/152, cero diferencias, fmt y Clippy `-D warnings`. El clon se retiró. `scripts/i07c_rd_conformidad.py` limpia `case.n` mediante `TemporaryDirectory`. Antes de estado final, ejecutar `python -B scripts/i11a_estado.py` para incluir los hashes SHA-256 actuales de ADR, handoff, pendientes, recovery, notas y artefactos. Revisión independiente, aprobación del propietario, I11b y puertas F0 permanecen pendientes.

## I11a val56 — actualización local (2026-11-09)

Las regresiones val56 están integradas; A ahora lexea escalares Unicode y reduce recursivamente tokens, B usa tokens de bytes y una pila, y producción conserva su recorrido de bytes con comprobación global de duplicados pre-esquema. Campañas nuevas 202611090901/debug/100000 y 202611090902/release/300000 incluyen 128 categorías, 302 variantes de esquema y 494139 mutaciones sistemáticas de documentos por perfil. La población completa registra 208 mutaciones: 192 muertas, 15 equivalencias, una deuda D2 y cero supervivientes no equivalentes; 41/41 `m56_*` muertas, SHA restaurado.

`sh scripts/validate.sh` en un clon limpio sin artefactos temporales ni `target`: exit 0, debug 1481/66, release 1483/67, Clippy `-D warnings` y fmt; RD debug/release 152/152 con cero diferencias. Clon y target retirados. No se cambió producción en esta ampliación. La revisión independiente posterior a val56, decisión de fase del propietario, I11b y puertas F0 siguen pendientes; regenerar el estado después de las actualizaciones documentales.

Estado regenerado tras esta actualización con `scripts/i11a_estado.py`; repetir esa regeneración tras futuras modificaciones documentales.

## I11a val57 — estado local vigente (2026-11-09)

La corrección cubre los nueve supervivientes val57 con regresiones en `tests/run_cli_diagnostics_and_cells.rs`. La suite de ejecución normal añade una campaña estructural determinista de 36.576 casos y 28 fronteras explícitas; las campañas independientes usan semillas 202611091201/202611091202 para 100.000/300.000 casos. La campaña documental completa comprueba 562.999 mutaciones por perfil, incluidas secuencias JSON y sustituciones de dos bytes. La matriz cells-64 cubre 0, 1, 2 y 30 registros Ok/Err/mezclados y fija Ok vacío = 2 celdas.

La población acumulada de 219 mutaciones queda en 203 muertas, 15 equivalentes, D2 como deuda y cero supervivientes no equivalentes; se verificó restauración SHA-256 por mutación. D3 registra que A2 §11.6 ll. 1063–1064 no fija si el literal de write/reply sigue vivo al reservar el resultado. Este registro supersede las métricas locales históricas de I11a anteriores a val57; revisar `Pending.md` y los artefactos vigentes de `docs/validation/`. La validación independiente, aprobación del propietario, I11b y puertas F0 permanecen pendientes.

Validación local final val57: `sh scripts/validate.sh` en una copia limpia sin artefactos temporales ni `target` iniciales salió 0. Suites: debug 1490/66 y release 1492/67; RD 152/152 en ambos perfiles, cero diferencias; fmt y Clippy `-D warnings`. La copia y su target se eliminaron. No equivale a revisión independiente ni aprobación del propietario.

## I11a val58 — continuación local (2026-11-10)

El R0001 de entrada a un intrínseco con recurso temporal ahora limpia el propietario por el camino único de `dispose_intrinsic_resources`; el evento `close:` convierte su reserva aunque el tick de entrada de `MockReadRecords`, `Inspect` o `Drop` sea el primero rechazado. `tests/runtime_profiles_and_resource_limits.rs` contiene 14 pruebas activas y un testigo pesado de `ir_items` ejecutado por campaña.

Reproducir `python -B scripts/i11a_campanas.py` (semillas 202611091301/1302) y `python -B scripts/i11a_mutaciones.py --quick-release` (población Release en copia desechable). Sin `--quick-release`, el script genera el artefacto del perfil debug y omite las comprobaciones adicionales de `from_id`, por lo que no reproduce el JSON versionado.

La corrección y esta evidencia son locales; la revisión independiente del estado val58 y la aprobación de fase del propietario permanecen pendientes. I11b y las puertas F0 siguen abiertas; no se creó commit.

Verificación limpia val58: `sh scripts/validate.sh` terminó en 0 con debug 1505/67 y release 1507/68; RD 152/152, cero diferencias, Clippy y fmt incluidos. Las campañas y la población completa se reprodujeron byte a byte con hashes `0a8dbe7b…` y `8749eaa6…`. Regenerar `docs/validation/F0-0002-I11a-estado.json` después de este HANDOFF y las notas; no implica revisión independiente ni aprobación.

## I11a val58 — revisión auxiliar posterior (2026-11-10)

La revisión auxiliar observó que `Drop` debe liberar el recurso antes de materializar su unidad bajo cells-64. El código ya seguía ese orden: `src/interp/machine.rs:1397–1405` dispone los argumentos antes de `1407–1412`, donde reserva la unidad. La nueva regresión `drop_destroys_resource_before_materializing_unit_at_the_cell_boundary` fija JSON literal, cierre y reserva cero; un mutante que adelanta la reserva muere en esa prueba. No se cambió producción por este hallazgo.

La documentación ya indica que los perfiles I10a/I10b están publicados y disponibles; D15/F0-CODIGOS los conservan como históricos para `implementation_limit`. Campañas reproducidas con SHA `0a8dbe7b…`; mutaciones 296 (280 muertas, 15 equivalentes, D2 deuda), SHA `6a1e761093cbeea40eab5808816f1bd700b48cf1cf950a930ae343b293e71256`. El auxiliar no es Claude Sonnet 5.5; revisión designada, I11b y aprobación del propietario siguen pendientes. No se creó commit.

`sh scripts/validate.sh` en clon efímero sin artefactos temporales ni `target` inicial terminó en 0: debug 1506 aprobadas/67 ignoradas; release 1508/68; fmt y Clippy `-D warnings`. RD directo con el binario debug recompilado: 152/152 check/run, cero diferencias de snapshot. Clon/target retirados; mínimo C libre 9,81 GiB. El target de trabajo se limpió; regenerar el estado antes de cerrar este corte.

La siguiente revisión auxiliar encontró IDs de perfil históricos en la tabla «Runtime experimental de F0-0002» de F0-CODIGOS; ya se sustituyeron por el perfil normal B y los ocho IDs B reducidos, y D15 §7.1 remite a §6.1. `profile_catalog_matches_official_b_cli_identifiers` comprueba los nueve IDs y la remisión; `w58_catalog_profile_old_id` muere al restaurar un ID antiguo. La población ahora contiene 297 mutaciones; la validación limpia final y el estado regenerado se anotarán a continuación.

Validación final posterior a sincronizar perfiles: debug 1507 aprobadas/67 ignoradas, release 1509/68; fmt y Clippy `-D warnings`. RD 152/152 y cero diferencias de snapshot. La población de 297 (281 muertas, 15 equivalentes y D2) tiene SHA `7645fa4fcb75439f8444d4cb1ddbd08629b4846987ddfabb8ec38e86e11aefd4`; campañas conservan `0a8dbe7b7312e5fd87ab0d72bc9d917cc0b498b3b26486588c4672a9f6a35849`. Clon y target eliminados; revisar estado/hash después de actualizar los documentos.

## I11a val59 — correcciones y verificación local (2026-11-10)

Se incorporaron seis regresiones en `tests/explain_and_print_output.rs` para las opciones y serialización de `explain`, formato de `Print` y cuotas publicadas. A2 §11.5 ll. 967–970 y §12.4 ll. 1594–1596 requieren comprobar adquisición más cierre antes de abrir; el caso con 42 bytes previos ahora fija `observed=89` y no 66. `explain` rechaza `--format` repetido según la forma única de A2 §13.10 l. 2210. Las menciones a D15 CLI apuntan a §7.1.

La población reproducida contiene 341 mutaciones: 324 muertas, 16 equivalentes justificadas, D2 como deuda y cero supervivientes no equivalentes; todos los SHA de producción se restauraron. Incluye los mutantes medidos de val59 y el caso de adquisición completa. Las campañas se reprodujeron dos veces, SHA-256 `0a8dbe7b7312e5fd87ab0d72bc9d917cc0b498b3b26486588c4672a9f6a35849`; el artefacto de mutaciones, SHA-256 `b3f17316dc3d71359ff025d637d67ac139adb8890afe38d1b453fd5ee70aab92`.

En clon limpio de validación, creado sin archivos temporales ni artefactos de compilación iniciales, `sh scripts/validate.sh` terminó con código 0: debug 1513/67, release 1515/68, fmt, Clippy `-D warnings` y RD 152/152 en ambos perfiles, cero diferencias de snapshot. El espacio libre observado al cierre fue 9,36 GiB; el clon y su target se retiraron. El estado final se regeneró después de este HANDOFF y del resto de documentos hashados. La revisión independiente posterior a estas correcciones, I11b, puertas F0 y decisión de fase del propietario siguen pendientes.

## I11a val60 — seguimiento de correcciones (2026-11-11)

`tests/analysis_profiles_and_quotas.rs` ahora ejercita el despacho CLI de los perfiles publicados y conserva la identidad seleccionada en diagnósticos estáticos, runtime, de entrada y de `--mock-env`. Una ruta JSON de `input_failure` que emitía `null` para UTF‑8 inválido/tamaño excedido quedó corregida y cubierta. Se corrigieron las citas de val60 y otras referencias sección/línea de I11a; la auditoría midió 178 pares válidos. La población completa de mutaciones Release está ejecutándose en serie, seguida por las campañas, validación limpia y actualización final del estado.

## I11a val60 — evidencia local (2026-11-11)

Población Release: 351 (331 muertas, 19 equivalencias justificadas, D2 y cero supervivientes no equivalentes); todos los SHA se restauraron. Las 18 mutaciones de literales/variantes de `from_id` se detectaron. Campañas idénticas en dos generaciones, SHA-256 `6e95dfd9dcb5ead8ee2835c94d2604e008cf0699499b312637a67d4809a57e8d`; artefacto de mutaciones reproducible, SHA-256 `5c9c6a6a5006ca372e91a645f6878e8cff97073efbcf4462a8eb640191a429a8`. Auditoría de citas incluyendo JSON: 210 pares, cero asignaciones erróneas. Clon limpio `validate.sh`: exit 0, debug 1516/67, Release 1518/68; fmt, Clippy y RD 152/152 en ambos perfiles, cero diferencias. Resta regenerar dos veces el estado después de actualizar todos los documentos.

La auditoría final, extendida al estado JSON y a su generador, verificó 232 pares sección/línea sin errores. Regenerar el estado después de este HANDOFF.

La preparación del siguiente tramo detectó que `corpus/objetivo/12_option_match.n` y `13_option_none.n` contienen `match move value`; F0-0003 §2.2 asigna la adquisición consumidora del scrutinee a J04. Se registró la decisión de alcance en Pending como NB-J02J03-01. J02 y J03 siguen pendientes, y el objetivo permanece en 10/29. Las descripciones de `explain` NX-E1325 y NX-E1326 se separaron por subcausa con una prueba dirigida; la suite de biblioteca y el test del perfil J01 pasaron. No se declara completado ningún J adicional.

El estado se regeneró de nuevo dos veces después de actualizar los documentos: bytes idénticos y once hashes documentales concordantes. Revisión independiente, aprobación del propietario, I11b y puertas F0 siguen pendientes.

## Corrección posterior a val61 — evidencia local (2026-11-12)

`tests/run_argv_dash_value_semantics.rs` fija siete casos de argumentos `run`, con fuente B válida y salidas literales. La población Release se reaplicó dos veces: 357 registros, 335 muertos, 21 equivalencias justificadas, D2 como deuda, cero supervivientes no equivalentes y restauración SHA-256 en todos. Los cuatro mutantes ejecutables nuevos murieron; las dos variantes de `--format` se confirmaron como equivalentes mediante la suite completa. SHA-256 de mutaciones `07fff7450c9cc2922c275351509b96768dcbebff4e85a3dc256dc6305c2283eb`; campañas repetidas con SHA-256 `6e95dfd9dcb5ead8ee2835c94d2604e008cf0699499b312637a67d4809a57e8d`.

`cargo test --locked --offline` y su variante `--release`, formato, Clippy `-D warnings` y `sh scripts/validate.sh` en un clon inicial sin metadatos Git, temporales ni `target` terminaron sin fallos. El clon comprobó debug 1523/67, release 1525/68 y RD 152/152 en ambos perfiles, cero diferencias de snapshot; se eliminó con su target. El estado JSON queda pendiente de su regeneración final repetida tras esta actualización documental. Revisión independiente, I11b, puertas F0 y aprobación del propietario siguen pendientes.

El estado se regeneró dos veces después de actualizar los documentos; las salidas fueron idénticas y los once hashes documentales internos coinciden con los ficheros.

## I11a val62 — cobertura y población local (2026-11-12)

`tests/cli_argument_and_unicode_edges.rs` añade nueve regresiones aisladas. La matriz `run_argv_matrix` comprueba 1.904 argv generados contra un oráculo documental de A2/D15/D4 más la decisión D5; `mock_oracle` recorre cada escape `\uXXXX`, surrogates aislados y las 1.048.576 parejas alto/bajo en claves y valores. Las pruebas dirigidas pasan en Release.

Población Release `--quick-release`, ejecutada de nuevo en segundo plano desacoplado: 399/399 registros, 370 muertas, 28 equivalencias justificadas, D2 como deuda, cero supervivientes no equivalentes, SHA restaurado en todos y 18/18 comprobaciones `from_id`. Hash de mutaciones `8bd129ebec20d79ca67683784589a3683ca37f1ce5d626a1c19275a2682b7f57`. Las campañas se repitieron byte a byte con hash `becb61067d6dcad6776db87384a2eef49e96e239f3789ccaa54703e5516e11e9`.

D5 resuelve en notas, sin enmienda normativa, el comportamiento no especificado para subcomandos desconocidos, `--` y valores vacíos. La siguiente fase de cierre es la validación completa y RD en un clon limpio, higiene/citas, notas de cierre y dos regeneraciones finales del estado. Sin commit, push ni acceso de red.

## I11a val62 — cierre local de calidad (2026-11-12)

`sh scripts/validate.sh` finalizó con código 0 en un clon limpio retirado al terminar: debug 1.535/0/67 y release 1.537/0/68; fmt y Clippy `-D warnings` completados. RD: 152/152 pares check/run por perfil, cero diferencias. Higiene: 0 hallazgos; citas 214 extremos, sin errores salvo los avisos conocidos del auditor para B-G1, validados manualmente bajo el subapartado B de §11.8.

La población Release `--quick-release` y las campañas se reprodujeron dos veces con SHA-256 idénticos `8bd129ebec20d79ca67683784589a3683ca37f1ce5d626a1c19275a2682b7f57` y `becb61067d6dcad6776db87384a2eef49e96e239f3789ccaa54703e5516e11e9`. Sin commit, push ni red; se regenerará dos veces el estado final después de este HANDOFF y RECOVERY. Revisión independiente, decisión de fase del propietario, I11b y puertas F0 siguen pendientes.

Programas objetivo que ejecutan: 12 de 29 (corpus/objetivo/, scripts/objetivo.py)

## I11b — cierre local de implementación (2026-11-12)

La matriz V01–V13 y el inventario de 68 pruebas `#[ignore]` están en `docs/validation/F0-0002-I11b-matriz.md`; las 68 tienen motivo explícito. Las pruebas ignoradas de integración pasaron (14 casos). La pasada Release de la biblioteca ejecutó las campañas ignoradas; los 7 fallos restantes son razones locales de escala no normativas y el test de razón temporal I10a, registrados como NB-I11b-01..08 en `Pending.md`. Los testigos de escala que rebasaban cuotas del perfil se redujeron conservando la relación 4×, sin cambiar cuotas de producción.

RD: 152/152 pares check/run en debug y release, cero diferencias de snapshot. Fuzz Release: parser, JSON mock y argv con 20.000 casos cada uno y diferencial mock de 100.000; cero pánicos, `internal_error`, `InvalidIr` o cuelgues. El diferencial CLI nuevo comprobó 5 fuentes válidas y 10 inválidas de corpus/pass y corpus/fail en ambos perfiles; check/run conserva estado y diagnóstico. `corpus/objetivo/` ejecuta 10 de 29 programas.

Las mediciones no normativas de I11b siguen en Pending. B1 quedó corregido en I11b; B2 y V06 se corrigieron después en F0-0007, con evidencia actualizada en la matriz.

Validación de suites I11b en clon limpio: `sh scripts/validate.sh` exit 0; fmt y Clippy `-D warnings`; debug 1.543/0/67 y release 1.545/0/68. Por separado, `scripts/i07c_rd_conformidad.py --verify` dio RD 152/152 check/run por perfil, cero diferencias; el diferencial nuevo de corpus pasa en ambos sentidos. Inventario de ignorados: 68/68 con razón; las 14 pruebas de integración pasan. Ocho observaciones de métricas locales no normativas están registradas en `Pending.md`. Objetivo actualizado: 10/29.

## Higiene de nombres de pruebas (2026-11-13)

Los nueve tests de I11a ahora usan nombres por comportamiento. `sh scripts/validate.sh` en copia limpia completó fmt, Clippy, debug 1.543/0/67 y release 1.545/0/68; RD dio 152/152 en ambos perfiles y cero diferencias de snapshot. Los cuerpos de prueba no cambiaron. El estado hashado se regenera tras esta actualización.

## F0-0007 — inferencia diferida de constructores `Result` (2026-11-13)

`src/check/types.rs` conserva hijos de `Result` en el grafo monomórfico y difiere NX-E0113 hasta cerrar las restricciones locales. La unificación de componentes es atómica; `AssertEq` relaciona los dos argumentos en orden fuente sin excepción sintáctica. `tests/i08.rs::deferred_result_alias_conflict_preserves_atomic_result_equality` reemplaza la expectativa defectuosa NX-E0113 de A/T01 por NX-E0102 con los dos tipos completos; el conflicto no fija T desde la igualdad rechazada. `tests/f0007.rs` cubre V1–V12, N1–N10, reparaciones, alias, anidación, ramas, bucles, continuidad tras conflicto y orden de recursos. La antigua prueba `adr_gap_result_component_mismatch_keeps_written_expected_type` se renombró y exige el tipo compuesto completo según F0-0007 §4 y A2 §13.17; no se abrió una laguna nueva.

Los 12 válidos aceptan en `check` y terminan en unidad sin eventos en `run`; los 10 inválidos dan exactamente el diagnóstico literal de F0-0007 §4. El diferencial de 69 fuentes del ADR y `corpus/pass`, `corpus/fail`, `corpus/objetivo` no encontró diferencias debug/release ni de diagnóstico `check`/`run`. RD dio 152/152 pares por perfil y cero diferencias de instantánea. El fuzz existente de parser y de Result terminó 20.000 casos por familia y perfil, con semillas 2026111307 y 0xf000700720261113, sin pánicos, `internal_error`, `InvalidIr` ni cuelgues. `scripts/objetivo.py --actualizar` conserva 10/29: los objetivos que faltan requieren capacidades posteriores, no solo esta inferencia. `sh scripts/validate.sh` terminó con código 0 en una copia limpia: formato, Clippy `-D warnings`, suites debug y release. La matriz V06 registra la evidencia local; la revisión independiente y la aprobación de fase permanecen pendientes.

La corrección adicional cubre el conflicto atómico de `Result` anidado, las dos obligaciones de cierre, alias con payload pendiente, `?`, payload `Never` y Array de Result pendiente en `tests/f0007.rs`. En el alias anidado sin componentes opuestos concretos, A2 §4.1.2.2 obliga a omitir NX-E0102 sin tipos serializables y a emitir NX-E0113 en cada constructor; no se inventan `expected_type` ni `found_type`. Un barrido de 450 programas de profundidad 1–4 con `Ok`/`Err` mixtos, alias abiertos y anotados, literales, `?` y Never, más ocho ciclos directos y por aliases, dio cero pánicos, `internal_error`, `InvalidIr` y diferencias `check`/`run` o debug/release. Frente a HEAD, hubo 152 cambios de rechazo a aceptación por el contexto posterior de F0-0007 R2 y ninguno en sentido contrario. La sonda de Result conserva la semilla original y añade otra, con 20.000 mutaciones por semilla. El avance objetivo sigue en 10/29.

NB-I11b-12 quedó corregido: la comprobación de ocurrencia impide enlaces cíclicos entre hijos `Array` y `Result`. La igualdad recursiva conserva dos NX-E0113 con rutas finitas literales; las regresiones cubren `Err`, aliases, varios nodos y Array intermedio. V06 figura PASA por evidencia local de B1–B5 y NB-I11b-12; la revisión independiente del corte y la aprobación de fase permanecen pendientes.

El residuo B4 de la verificación dirigida se corrigió: `?` sobre un binding `Result` pendiente proyecta los hijos `ok`/`err` del mismo representante monomórfico, y su error se unifica con el de la función. `tests/f0007.rs` fija la aceptación del caso con `r?`, alias, contexto anterior y posterior, ramas, bucles y `TryAdd`, y el NX-E0102 literal cuando el error concreto difiere. La nota de N10 en F0-0007 decide que `Err(Ok([]))` emite solo NX-E0103 por el Array; la regresión fija su primario y sus datos.

La reasignación de `var` reveló otra vía de la misma corrupción de forma: la unión de un representante con hijos `Result` y otro con un escalar concreto. `union_inner` comprueba esa incompatibilidad antes de enlazar o ligar hijos; `union` restaura atómicamente la igualdad fallida, y la resolución no oculta hijos estructurales tras un escalar. El repro `fn Main(){let a=Ok(1);var r=Ok(a);r=Ok(1);r=Err(true);}` y su variante `Err(1)` rechazan en `check` y `run` con cuatro NX-E0113 literales, sin error interno (A2 §§4.1.2.2, 13.17; F0-0007 R3/R5). `tests/f0007.rs` añade reasignaciones Result/Array anidadas, compatibles e incompatibles, alias, ramas, bucles y después de `?`, con NX-E0102 literal cuando el conflicto es serializable. Los barridos de 450 y 550 casos se repitieron y se añadieron 300 reasignaciones: cero fallos internos, divergencias debug/release o `check`/`run`, y cero aceptaciones de HEAD convertidas en rechazo.

La verificación dirigida repitió 450 programas anidados y añadió 100 con `?` sobre bindings: cero divergencias de diagnósticos o aceptación entre `check`/`run` y debug/release, cero fallos internos y ninguna aceptación de HEAD convertida en rechazo. Los 100 nuevos programas aceptan; en total, 252 rechazos de HEAD pasan a aceptación. Formato, Clippy, suites debug/release y RD 152/152 por perfil finalizaron correctamente. `sh scripts/validate.sh` pasó en una copia limpia sin archivos temporales ni `target` iniciales; la copia y su `target` se retiraron. El corpus objetivo sigue en 10/29.


## F0-0003 — J01 y unión nominal compartida (2026-11-13)

La CLI acepta `f0-0003-b-v1` y sus ocho perfiles reducidos por el selector vigente; el perfil publica `effect_model=f0-0003-sim-v1`, el catálogo/schema y el manifiesto de intrínsecos pendientes. Las capacidades J posteriores conservan barreras `NX-E0006/implementation_limit` con su ámbito, mientras los perfiles A2 mantienen sus diagnósticos y ejecución. El descriptor nominal compartido fija Result Ok/0 y Err/1, y Option None/0 y Some/1; el unificador transaccional usa hijos por variante, la IR y ambos verificadores usan tag/payload, y el intérprete almacena una unión con un solo payload activo. El lowering de Result conserva los costes, la propiedad y la limpieza de A2.

El diferencial frente a HEAD construido con `git archive` comparó 2624 fuentes y 10648 pares de invocaciones de `check`, `run` y `explain` en debug/release: cero diferencias en stdout, stderr y código de salida. RD dio 152/152 por modo y cero diferencias de snapshot; la suite release, formato y Clippy `-D warnings` pasaron. Fuzz release con semilla nueva: 20.000 casos de parser y 20.000 de tipado, sin pánicos. La pasada única de cinco mutantes de reglas Result detectó 4; 1 sobrevive sin programa causal registrado. `corpus/objetivo/` continúa en 10/29: esta preparación no activa aún Option ni match. La revisión independiente del corte y la aprobación de fase por el propietario siguen pendientes.

## F0-0003 — implementación J02, J03 y J04 (2026-11-13)

J02 habilita `Option`, `Some` y `None` con inferencia contextual, copia y destrucción transitivas y O01/O02 sobre el descriptor nominal compartido. J03 habilita `match` de lectura, cobertura simbólica, unificación de brazos, M01–M05, M07/M08, M10/M11 y el binding `None` bajo `Result`. J04 incorpora selección `move` y `edit`, extracción y permisos de patrón, joins M06/M09, escritura a través de Place indexado y limpieza de recursos en `return` y `?`. Las guardas siguen bajo J05; closures, generadores y `for` conservan sus barreras.

Los programas objetivo 12 y 13 ejecutan con resultado unitario, sin salida, eventos ni diagnósticos, conforme a F0-0003 §§3, 6 y 11. El medidor registra **12 de 29**, sin pérdida de los diez anteriores. La suite estructural cubrió 500 fuentes `Option`/`match` anidadas y el fuzz release 20.000, sin pánicos ni errores internos. El diferencial de 248 fuentes J, corpus y RD ejecutó 992 comandos debug/release y `check`/`run` sin diferencias; el diferencial con HEAD sobre 1.590 fuentes ajenas a `Option`/`match` comparó 6.360 comandos byte a byte con cero diferencias. RD conserva 152/152. `sh scripts/validate.sh` terminó con código 0 en una copia limpia sin archivos transitorios, con formato, Clippy `-D warnings` y ambas suites offline/locked.

La pasada única rápida de cinco mutantes de núcleo detectó cuatro y restauró los cinco archivos por SHA-256. El superviviente `option_none_coverage` se debió a que su test rápido M01 no ejercita el binding `None` bajo `Result`; NB-J04-01 registra el programa discriminante ya cubierto por una regresión y la comprobación dirigida pendiente. La revisión independiente, las puertas F0 y la aprobación de fase por el propietario siguen pendientes.

## F0-0003 — corrección de patrones y temporales (2026-11-13)

La resolución de nombres exige que un constructor de patrón sea una variante nominal y emite NX-E0101/call_target para funciones e intrínsecos ajenos. El checker concreta el tipo del scrutinee temporal antes de comprobar patrones; el parser incorpora los bindings del brazo a su alcance léxico, incluido el sombreado de una función. El último brazo de la IR dirige la ruta falsa a un bloque terminal con limpieza, de modo que el verificador conserva la prueba de variante en la ruta que proyecta el payload.

`tests/j04_match_semantics.rs` fija los cuatro defectos con pares positivos y negativos; `src/ir/verify.rs` comprueba además una mutación del tag que debe rechazar la proyección. `explain` publica ejemplos B pareados para NX-E1304, NX-E1325 y NX-E1326. Dos barridos release de 20.000 fuentes, con semilla 202611131730, terminaron sin pánicos, `internal_error`, `InvalidIr` ni cuelgues. El diferencial J comparó 264 fuentes y 1.056 invocaciones debug/release y check/run sin diferencias; RD conserva 152/152 en ambos modos. El diferencial byte a byte contra HEAD comparó 1.594 fuentes sin Option/match y 6.376 invocaciones con cero diferencias. El corpus objetivo sigue en **12 de 29**. Los puntos de andamiaje pendientes constan en Pending; la revisión independiente y la aprobación de fase continúan pendientes.

`sh scripts/validate.sh` terminó con código 0 en una copia limpia en D:, con formato, Clippy `-D warnings` y suites debug/release offline/locked.

## F0-0003 — diagnóstico de scrutinee inline irresuelto (2026-11-13)

El checker omite el diagnóstico derivado de un literal de patrón cuando su tipo aún es diferido; el constructor común de incompatibilidad impide emitir un tipo esperado vacío. Cada una de las tres fuentes `match Ok(1){Ok(1)…}`, `match None{Some(1)…}` y `match Err(true){Err(true)…}` devuelve solo NX-E0113, sin elementos `null` ni errores internos en la CLI. `tests/j04_match_semantics.rs` conserva positivos con tipos determinados y comprueba `check`/`run` JSON. El fuzz release de 20.000 fuentes con semilla 202611132133 verificó también la serialización de cada diagnóstico rechazado, sin pánicos ni fallos internos.

Formato, Clippy `-D warnings`, suites debug y release y `sh scripts/validate.sh` en copia limpia de D: terminaron con código 0. RD conserva 152/152 y cero diferencias de snapshot en ambos modos. El diferencial J midió 273 fuentes y 1.092 invocaciones sin diferencias; contra HEAD, 1.594 fuentes sin `Option`/`match` y 6.376 invocaciones byte a byte dieron cero diferencias. El corpus objetivo permanece en **12 de 29**. No se ejecutó otra mutación. La revisión independiente y la aprobación de fase continúan pendientes.
