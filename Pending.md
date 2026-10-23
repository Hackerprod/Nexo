# Nexo — Pendientes

## Estado
- Base 0.2 recibida y preservada. Acuerdos posteriores: docs/F0_ACUERDOS.md.
- El propietario asigna implementación al asistente y validación a subagente separado.
- F0: EN EXPLORACIÓN. F1–F7: NO APROBADAS.
- Documentos respaldados en c4f020b; primer corte ejecutable con revisión independiente PASS limitada.

## Siguiente corte
1. Leer HANDOFF.md; comprobar respaldo remoto de F0-0001 y reproducir pruebas.
2. Sustituir el frontend de sintaxis por la sintaxis oficial (candidata B, especificada en docs/adr/F0-D15-sintaxis-oficial.md, con PASS de diseño) en un solo corte, sin compatibilidad con la grafía anterior; I07b fue validada y publicada.
3. Mantener el registro de regresiones de cada revisión antes del respaldo verificado del corte.

## Evidencia del primer corte
- Parser, checker de tipos/propiedad, intérprete de AST y CLI check/run.
- 22 tests de integración (19 semánticos/corpus y 3 CLI), debug y release.
- fmt y clippy estricto pasan. Corpus: 5 positivos y 10 negativos por código.
- La revisión detectó altura compuesta del AST y precedencia de límites negativos;
  y truncamiento UTF-8 al limitar la lectura, corregidos con regresiones. Ver docs/validation/F0-0001-review.md para veredicto.
- Recursos simulados, copia profunda física y límite explícito de trabajo.
- En aquel primer corte no había IR bajada, préstamos, efectos, closures ni generadores ejecutables.

## Diseño F0-D11/D12/D13
- ADR D11 (closures), D12 (async), D13 (expresiones/módulos) y catálogo F0-CODIGOS: PASS de diseño en
  iteración 7 (docs/validation/F0-D11-D13-design-review.md). Sin ejecutar; los límites diferidos sobre atributos, tuplas, módulos y activación local de `extend` siguen en el ADR §16.
- ADR F0-0002 (llamadas, IR, préstamos, efectos, Result/?): PASS de diseño en iteración 6 (docs/validation/F0-0002-design-review.md). La enmienda posterior (partición I07a/b/c, cuotas `f0-0002-v2`, tipos diferidos y lagunas) y la fusión del catálogo de códigos en F0-CODIGOS.md pasaron revisión independiente de diseño.
- Las subtareas I01–I11 de F0-0002 §17 siguen el orden y las puertas del ADR.

## Decisiones y puertas aún pendientes
- D11–D13: ADR redactados y con PASS de diseño (no ejecutados, no aprobados por el propietario).
- Protocolo explícito de la propuesta H-PY 10×/3×; no resultado medido.
- D01–D10 completos, estados D09, coherencia de traits, gramática completa e IR/backend.
- Protocolos T01–T28/H-*/T26, plataformas y herramientas de referencia.
- Dos revisores sin discrepancias conforme a F0; un subagente no equivale a una
  auditoría externa ni elimina otros requisitos del roadmap.
- EX01/EX04 completos, generadores y biblioteca real corresponden a F2.

No ocultar pendientes al cambiar de workspace. No publicar secretos ni usar force push.

## Corte I01
- Estado por subtarea: I01 validada tras revisión independiente y publicada en `d4836cf`; I02a validada tras revisión independiente y publicada en `f073275`; I02b validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada en `f5016d7`; I05 validada tras revisión independiente y publicada en `a203e51`; I06 validada tras revisión independiente y publicada en `0eaf876`; I07a validada tras revisión independiente y publicada en `9f96780`; I07b validada tras revisión independiente y publicada; I08–I11 pendientes.
- El addendum de códigos quedó fusionado en F0-CODIGOS.md y eliminado; la enmienda de F0-0002 y F0-CODIGOS pasó revisión independiente de diseño.
- Diseño D11–D13 y F0-0002 con PASS de diseño; no equivale a aprobación del propietario.
- I01 abarca esquema, validación, JSON/texto, explain y lectura acotada; está validada tras revisión independiente y publicada en `d4836cf`. Las puertas de F0 siguen pendientes.
- Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.
- Límites conocidos de I01: las reglas de emisión y las inserciones vacías del mismo punto se comprueban en los emisores; los valores de cuota del perfil normal y la relación de `observed` como obligación del emisor constan como límites del esquema.

## Corte I02a
- Estado: léxico, tokens y spans validados tras revisión independiente y publicados en `f073275`.
- El lexer propuesto vive en `src/frontend/lex.rs`; el frontend de `syntax.rs` y la salida actual de `check`/`run` siguen separados.
- Incluye símbolos, identificadores/reservadas, enteros, decimal reducido, cadenas reconocidas, comentarios `//`, saltos LF/CR/CRLF y NX-E0001 léxico. LX01–LX05 tienen negativos y positivos pareados en la suite unitaria.
- I02b (parser, terminadores sintácticos, AST e impresor) está validada tras revisión independiente y publicada en `84d5f53`. L01–L07 y el resto de §14 no se cerraron en I02a.
- Ambigüedades de literales entre comillas y diagnóstico de lexema incompleto quedan registradas en `docs/validation/F0-0002-I02a-implementation-notes.md`; no se implementa decodificación semántica de cadenas.
- I02b ya está publicada; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada en `f5016d7`.

## Corte I02b

I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada en `f5016d7`; I05 validada tras revisión independiente y publicada en `a203e51`; I06 validada tras revisión independiente y publicada en `0eaf876`; I07a validada tras revisión independiente y publicada en `9f96780`; I07b validada tras revisión independiente y publicada; I08–I11 pendientes. El frontend propuesto sigue separado de `compile`, `check` y `run`. Los límites estructurales de análisis y las regresiones de robustez se describen en `docs/validation/F0-0002-I02b-implementation-notes.md`.

Pendientes nominales: enumerar todas las posiciones recursivas de `PlaceRoot`, listas/mapas/restricciones de `WithClause` y alternativas inválidas de `Statement` y `Program` más allá de los 55 prefijos comprobados; completar negativos aislados para cada continuación y contexto de terminador, incluida no inserción estructural en LF/CRLF/CR/EOF. Conservar las lagunas `adr_gap_*` enumeradas en las notas de I02b. El fallo de creación del hilo ancho tiene camino diagnóstico sin panic; su alcance excepcional está documentado en las notas. Las obligaciones semánticas y de emisión posteriores a I03 siguen abiertas. Las puertas generales de F0, en particular dos revisores sin discrepancias y aprobación del propietario, permanecen pendientes.

## Corte I03

I03 validada tras revisión independiente y publicada. El módulo aislado `check::check_proposed` cubre registro de firmas, duplicados, ámbitos, sombreado, asociación estructural, modos estructurales, entrada básica, literales y tipos independientes; cierra los negativos L01 y L07 en las etapas que asigna §17.2; sus positivos completos siguen asignados a pasos posteriores. La ruta heredada de `check`/`run` conserva su comportamiento. Consultar `docs/validation/F0-0002-I03-implementation-notes.md` para evidencia y límites.

Pendientes nominales: cierre completo de los positivos y observaciones de S01–S34 según §17.2; ejecución y propiedad de argumentos en I05–I06; análisis de caminos y valores de ramas en I07; semántica completa de Result, `?`, inferencia de constructores y puerta completa de entrada en I08; efectos en I09; cuotas completas en I10; integración en I11. Permanecen la fusión del addendum de códigos, dos revisores sin discrepancias y la aprobación del propietario para F0.

La duración actual de la suite completa y las razones de escalado de 256 KiB a 1 MiB constan en las notas de I03; la prueba preexistente de escalado del parser domina la duración. Límites conocidos: detección sintáctica de `resource_array` por nombre de intrínseco, guardas defensivas de profundidad con contadores nominales, inferencia incompleta de `Ok(Ok(1))`, mensaje semántico genérico, contexto de tipo esperado en arrays heterogéneos y tipo diferido de `?`. Las cuatro interpretaciones locales de cardinalidad de §13.9 tienen pruebas `adr_gap_*` y se describen en las notas.

## Corte I04a

I04a validada tras revisión independiente y publicada. IR escalar verificada, identidades y hash de funciones, tabla sustituible en desarrollo, bindings, reasignación completa, aritmética comprobada, comparaciones, `print`, `assert_eq`, `drop` de datos, `fail` y registro ordenado de salida están aislados de los comandos actuales. Las pruebas locales de IR y ejecución se enumeran en `docs/validation/F0-0002-I04a-implementation-notes.md`; `cargo test --locked --offline --lib -- --list` enumera 257 pruebas, incluida una medición de escalado de lowering de ejecución explícita. Los resultados locales no son revisión independiente.

Pendientes nominales tras I04a: I05 introdujo frames, llamadas de usuario, recursos, retorno y limpieza secuencial; I06 implementa préstamos localmente y espera revisión independiente. I07–I09 conservan CFG, Result y efectos; I10 debe cerrar cuotas y contabilidad completa; I11 integra CLI y flujos. Los positivos de §14 que requieren esas capacidades no se atribuyen a I04a. Permanecen los requisitos generales de F0, incluidos dos revisores sin discrepancias y aprobación del propietario.

## Corte I04b

I04b validada tras revisión independiente y publicada en `f5016d7`. La IR verificada y el intérprete aislado cubren arrays de datos, copias profundas, lectura y escritura indexadas, `len`, igualdad estructural y observación; las cuotas R0002 de celdas incluyen cabeceras y elementos. El negativo `immutable_write.nx` emite NX-E0105 en ownership con raíz y declaración relacionadas; los sitios independientes se acumulan en orden de fuente. `nested_copy` ejecuta copias sin alias. `drop(a)` libera el binding antes de crear la unidad; las lecturas indexadas prestan el elemento y las asignaciones o declaraciones propietarias copian. `docs/validation/F0-0002-I04b-implementation-notes.md` enumera las filas de §17.2, evidencia local y lagunas. La suite enumera 311 pruebas de biblioteca; la revisión separada y las puertas de F0 siguen pendientes.

Límites nominales conservados de I04b: ticks de `work_consumed` frente a §11.6 e `ir_items` corresponden a I10. El span de R0002 de la unidad final implícita cubre el cuerpo, no la `}`; el ADR no fija span de R0102 ni `expected_type` relacionado para asignación indexada. I05 añade el rechazo NX-E0202 tras `drop` y comprueba el código posterior a `return`. I06–I11 conservan permisos exclusivos, CFG, Result, efectos, cuotas completas e integración. El la puerta de dos revisores sin discrepancias sigue abierta.

## Corte I05

I05 validada tras revisión independiente y publicada. `ir::compile_scalar` y `interp::run_scalar` siguen aislados de los comandos actuales. La IR incorpora parámetros ordinarios y `consume`, llamadas directas por `FunctionId`, recursos mock y planes de limpieza; la ejecución conserva el orden escrito de argumentos nombrados y cierra cada recurso una vez. El verificador coteja las firmas referenciadas con la tabla de funciones, modos, propietarios, scopes, orígenes y spans. Los diagnósticos de copia prohibida, uso tras consumo o destrucción y elevación de permiso de lectura se comprueban en ownership. La revisión separada debe contrastar con sus propios oráculos todos los pares de F0-0002 §17.2.

`docs/validation/F0-0002-I05-implementation-notes.md` registra evidencia local, decisiones de hash, lagunas y reproducción. Los límites nominales `work_consumed` (§11.6), `ir_items` (I10a), span de R0002 para unidad implícita y precisión de R0102 siguen visibles. I06 incorpora préstamos exclusivos y fue publicada en `0eaf876`; I07a cubre CFG acíclico y bucles con límite nominal de punto fijo; I07b conserva propiedad sobre retroaristas, e I08–I11 conservan `Result`, efectos, cuotas completas e integración. F0 sigue pendiente de dos revisores sin discrepancias y aprobación del propietario.

La corrección de I05 añade regresiones para ids de entidades, orden inverso de temporales tras fallo de argumento, dependencias de `let` fallido, escala de firmas y funciones, y restauración barata del código inalcanzable con búsqueda de declaraciones sombreadas. La lectura indexada de una raíz consumida o destruida devuelve NX-E0202 con el sitio causal. I06 acumula dos errores independientes de propiedad en los argumentos de una llamada y convierte `f(r,consume r)` en NX-E1105/`active_borrow_conflict`; los casos de nombres y tipos siguen sujetos a la cardinalidad de §13.9. I10b conserva el span R0001 de unidad implícita `[9,len)` en vez de `}` y el tick ausente de `{}`. I05 fue validada tras revisión independiente y publicada.

## Corte I06

I06 validada tras revisión independiente y publicada en `0eaf876`. La IR y el intérprete aislados cubren préstamos de lectura y `inout`, exclusividad conservadora por raíz, représtamos, retorno de copias independientes, modificación por llamada y restauración de propietarios tras fallos. El préstamo `inout` indexado no añade celdas; P07 señala el lugar completo del ADR. S06, S08, S30, P05–P07 con `data` exacta y PP02 tienen pruebas locales. Ver `docs/validation/F0-0002-I06-implementation-notes.md` para evidencia y límites. I07a está validada tras revisión independiente y publicada en `9f96780`; I07b validada tras revisión independiente y publicada; I08–I11 siguen pendientes; la fase F0 conserva dos revisores sin discrepancias y aprobación del propietario.


## Corte I07a

El modo de análisis sin emisión se activa después de expresiones y condiciones divergentes, y en las colas inalcanzables. Las regresiones locales cubren asignaciones e índices posteriores, operandos y argumentos pendientes, ramas y cuerpos de bucle, retorno, `value:` y consumo de recursos. Los errores de valor de rama ausente se comprobaron con código, subcausa, etapa, operación, spans y `data` exactos en varias posiciones, ramas y sitios independientes. La matriz combinatoria exhaustiva de posiciones y propiedad sigue pendiente.

I07a validada tras revisión independiente y publicada en `9f96780`. Ramas, valores de bloque y bucles tienen CFG y ejecución aislada; P08, NX-E1317 y los positivos de L02/L05/L06 tienen comprobaciones locales. El verificador rechaza ciclos forjados y termina con un presupuesto de análisis; las regresiones cubren restauración de estado, vida de temporales entre aristas y dominancia en bucles. Diez familias con divergencia y controles vivos midieron pasos, bytes y asignaciones de 256 KiB a 1 MiB con razón máxima 4,398; una mutación cuadrática fue detectada y restaurada por SHA-256. Faltan negativos aislados y positivos para todas las combinaciones de posición, tipo, rama y propiedad, así como cardinalidad combinada; el fuzz dirigido y la campaña de mutaciones del corte anterior constan en las notas. P09 y cualquier unión de propiedad que dependa de un punto fijo en retroaristas permanecen en I07b, con rechazo nominal `OutsideScalarSlice("ownership fixpoint")` mientras tanto. Las obligaciones de `Result`/`?` (I08), efectos (I09), cuotas y contabilidad completas (I10), integración (I11), dos revisores sin discrepancias y aprobación del propietario siguen abiertas. La revisión independiente de I07b sigue pendiente. Ver `docs/validation/F0-0002-I07a-implementation-notes.md`.

El negativo aislado del operando derecho `()` y la prioridad estructural de `value:` tienen regresiones locales con codigo, subcausa, span, token y campos exactos. Falta ampliar la matriz a combinaciones no muestreadas.

El corte anterior de I07a conserva regresiones para los nueve repros bloqueantes y añade una medición de tres formas de `while` con condición ramificada y retroaristas; pasos y tráfico crecen con razón máxima 4,335 entre 256 KiB y 1 MiB. La mutación del barrido cuadrático fue detectada y restaurada por SHA-256. La validación local y la comparación con HEAD terminaron en 0, con `0 differences`; los listados de biblioteca contienen 580 pruebas en debug y 582 en release, incluidas 11 mediciones ignoradas por perfil. Estado local: INCOMPLETA. El fuzz dirigido de 100.000 fuentes con oráculo propio, 42 mutaciones nuevas y las supervivientes nominales quedaron detectados y documentados. Pendientes nominales: negativos aislados con campos exactos de las combinaciones restantes de divergencia, propiedad y tipo diferido sin anotación. I07a validada tras revisión independiente y publicada en `9f96780`; las puertas de F0 siguen abiertas.


El resultado de expresion distingue `Flow::Value { temp: Option<TempId>, ty }` de `Flow::Diverged { ty: Option<Ty> }`. Literales, nombres, indices, operadores, llamadas, arrays, `if` y bloques de valor usan ese resultado en el recorrido vivo y analizado. `emit` devuelve un temporal opcional. Quedan nueve apariciones de `usize::MAX`: siete son destinos CFG aun sin enlazar, una es el retorno ficticio de `finish_block` durante el analisis y una pertenece a un assert de test. Ninguna decide la divergencia de expresiones. Los usos de `== Ty::Never` comprueban el tipo interno inferior de bindings sin anotacion cuyo valor inicial diverge; no son centinelas de flujo. El tipo de los operadores procede de su operador y operandos presentes, con independencia del lado divergente. Un `()` derecho de suma o resta numerica da NX-E0102/type_mismatch con `expected_type=i64` y `found_type=()`. La decision `adr_gap_never_bottom` trata `Never` como fondo del reticulo: en igualdad y arrays no impone `i64`; el primer operando concreto fija el tipo y el diagnostico recae sobre el que lo contradice. La prioridad de NX-E1302/missing_branch_value se aplica a la funcion en el mismo recorrido vivo y analizado.

La regla vigilada de §13.9 conserva cada sitio independiente con izquierdo divergente: el error numérico del operando derecho se emite en el checker y se ordena por span junto con los demás errores de la función. La comprobación estructural de `value:` conserva prioridad cuando falta un valor de rama. El oráculo local comparó 300.000 pares de 67 plantillas, incluidas llamadas, arrays, binarios y hasta cuatro errores por función, con código, subcausa, etapa, operación, `data` y spans mapeados: cero diferencias en esa muestra, cero `Never` en `data` y ningún fallo interno. La muestra no acredita cobertura exhaustiva. La medición previa de 76 familias registró razones máximas de 4,14 en bytes y asignaciones; el nuevo recorrido de errores requiere una medición independiente de escala.

`cargo test --locked --offline --lib -- --list` enumera 607 pruebas en debug y 609 en release, con 12 ignoradas por perfil; las activas son 595 y 597. La batería `ir::` contiene 349 activas y 12 ignoradas; `check::` contiene 98 activas. Las veinte mutaciones nuevas de las reglas de operandos y `Never` fueron detectadas en debug y release con restauración SHA-256. I07a validada tras revisión independiente y publicada en `9f96780`. Siguen abiertas la matriz combinatoria exhaustiva, la revisión independiente y las puertas de fase del propietario. `sh scripts/validate.sh` terminó en 0 y `sh scripts/compare-head.sh` devolvió `0 differences`.

Los límites `adr_gap_deferred_var_alias` y `OutsideScalarSlice("lowering depth")` constan en las notas de I07a. I07a validada tras revisión independiente y publicada en `9f96780`.

## Corte I07b

I07b validada tras revisión independiente y publicada. P09 y la propiedad en retroaristas tienen análisis disperso de punto fijo. Las salidas del `while` comparten los efectos de la condición completa; el resultado literal de un brazo no selecciona el estado de propiedad. La cola por índice estabiliza la cabecera antes de los bloques posteriores. Al alcanzar el límite por función se conservan diagnósticos de estados estables, con una entrada por uso; se excluyen estados intermedios y las funciones siguientes se analizan.

Quedan como límites nominales la ausencia de evaluación de valores en comparaciones, las cascadas de propiedad heredadas, una cadena causal directa duplicada y la falta del perfil reducido en la CLI. Programas válidos de 512 KiB con bucles anidados o condición `if` expresión pueden agotar la cuota normativa; la cadena de consumos y bucles alcanza K=11.759, unos 743 KiB. La forma con todas las declaraciones consumidas antes de los bucles agota la cuota y conserva `maybe_consumed` junto a NX-E0003; la forma intercalada no la agota. La contabilidad visible de §11.8 corresponde a I10a y requiere enmienda del ADR respecto a la expectativa de 65 KiB válidos. I08–I11, la revisión independiente, dos revisores sin discrepancias y la aprobación del propietario siguen pendientes. Las reglas y la evidencia están en `docs/validation/F0-0002-I07b-implementation-notes.md`. I07a está publicada en `9f96780`; el corpus independiente de terminaciones está en `45886fd` y CI en `f6f94b9`/`97257cd`.

El listado de biblioteca contiene 673 pruebas en debug y 675 en release, con 19 ignoradas por perfil; `ir::` ejecuta 408 activas en debug y 408 en release. `sh scripts/validate.sh` terminó en 0 en el árbol principal y en la copia LF; `sh scripts/compare-head.sh` devolvió `0 differences`.
