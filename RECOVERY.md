# Recuperación de Nexo

1. Leer README.md, Pending.md y docs/F0_ACUERDOS.md desde el estado remoto actual.
2. Leer íntegra la base 0.2 y comprobar los hashes indicados en F0_ACUERDOS.md.
3. Inspeccionar git status y el último commit remoto; no sobrescribir cambios de otro
   agente ni asumir que un archivo local llegó a GitHub.
4. Continuar únicamente el siguiente corte declarado. Toda fase sigue pendiente
   salvo evidencia y revisión explícitas; no confundir compilación con validación.
5. Implementador y validador deben estar separados. Guardar hallazgos y regresiones.
6. Respaldar cada paso completado en GitHub con commit descriptivo y verificar el
   nuevo HEAD. Nunca reescribir el historial para hacer desaparecer fallos.

Los documentos originales del usuario están en docs/baseline/. Las recomendaciones
posteriores y sus estados están en docs/F0_ACUERDOS.md. No reconstruir decisiones
basándose solo en resúmenes de chat.

Al primer respaldo documental no se publica ni se afirma validado el código parcial
que pueda existir en un workspace. Su publicación requiere alcance y pruebas propios.

## Reproducir el primer corte

Usar la cadena de `rust-toolchain.toml` con rustfmt/clippy. Ejecutar
`scripts/validate.sh` desde la raíz del repositorio; los tests no necesitan red
ni dependencias externas. Los artefactos de `target/` no se respaldan y no son
prueba de que el código actual haya pasado. Recompilar antes de repetir repros.
La revisión independiente y hashes están en `docs/validation/F0-0001-review.md`.
No ocultar los tres defectos originales encontrados durante la revisión.

## Recuperación del corte I01

Leer primero README.md, Pending.md y docs/F0_ACUERDOS.md. Comprobar los hashes de la base contra el blob de Git mediante `git show HEAD:<ruta> | sha256sum`; inspeccionar `git status` y el último commit remoto antes de continuar. Continuar solo el siguiente corte pendiente. Mantener separados implementador y validador, respaldar cada paso con commit descriptivo y verificar HEAD remoto sin force push. Los tres defectos originales del primer corte (altura compuesta del AST, orden de límites negativos y truncamiento UTF-8) permanecen documentados y no deben ocultarse.

I01 está validada tras revisión independiente y publicada en `d4836cf`; I02a está validada tras revisión independiente y publicada en `f073275`; I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada en `f5016d7`; I05 validada tras revisión independiente y publicada en `a203e51`; I06 validada tras revisión independiente y publicada en `0eaf876`; I07a validada tras revisión independiente y publicada en `9f96780`; I07b validada tras revisión independiente y publicada; I08–I11 pendientes. El addendum de códigos sigue pendiente de fusionar. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.

## Recuperación de I02a

I02a está validada tras revisión independiente y publicada en `f073275`. El lexer nuevo está aislado en `src/frontend/lex.rs` y se exporta bajo `frontend::lex`; no se ha conectado a `compile`, `check` o `run`. Consultar `docs/validation/F0-0002-I02a-implementation-notes.md` para alcance, lagunas y suites. I02b está validada tras revisión independiente y publicada en `84d5f53`; el frontend propuesto sigue aislado de la CLI hasta el paso de integración asignado por el ADR.

## Recuperación de I02b

I02b está validada tras revisión independiente y publicada en `84d5f53`. `frontend::parse` y `frontend::print` siguen aislados de `check`/`run`; el estado técnico, las lagunas del ADR, las pruebas y los límites conocidos están en `docs/validation/F0-0002-I02b-implementation-notes.md`. Antes de continuar, comprobar `git status`, comparar con el remoto según el flujo de agentes y ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`. La aprobación de fase F0 sigue reservada al propietario.

La terminación se decide al descender por cada sentencia y los diagnósticos FIRST usan el estado de la producción comprometida. El inicio de cada sentencia se guarda durante el descenso; los operadores excluidos conservan su prefijo exacto tras LF, CRLF, CR y comentarios. El parser usa un hilo ancho reutilizable incluso para anidamientos bajos, con diagnóstico si falla su creación. La suite de regresión incluye prefijos repetidos, `?`, sombreado, ramas de retorno, fronteras de presupuestos, lista de operaciones vacía, inserción tras trivia y pilas pequeñas mediante subprocesos. Revisar los límites nominales de posiciones FIRST y terminadores antes de cerrar I02b.

## Recuperación de I03

I03 está validada tras revisión independiente y publicada. Revisar `src/check/names.rs`, `src/check/types.rs` y `src/check/tests.rs` junto con `docs/validation/F0-0002-I03-implementation-notes.md`. El API `check::check_proposed` es aislado y devuelve una lista ordenada de diagnósticos semánticos; `compile`, `nexo check` y `nexo run` siguen usando la ruta anterior. Repetir los tests locked/offline, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`. La salida del comparador debe terminar en `0 differences`. Registrar cualquier fallo con una regresión antes de ampliar el alcance; no atribuir a I03 ejecución de llamadas, préstamos, efectos, control por caminos ni Result completo. I03 pasó revisión independiente en su tercera ronda.

Al retomar I03, repetir primero la prioridad entre nombres y argumentos, los spans de anotación acabada en `>`, el salto que terminó `return`, la cardinalidad de errores independientes, las ramas que retornan, el tipo diferido de `?`, la sonda de pilas pequeñas y la restauración de ámbitos y contexto de tipos. La prueba de escalado usa 16 familias por defecto y 24 con `NEXO_FULL_SCALE`, de 256 KiB a 1 MiB. La duración actual está en las notas de I03; la prueba de escalado preexistente del parser domina la suite.

## Recuperación de I04a

I04a validada tras revisión independiente y publicada. Revisar `src/ir/model.rs`, `src/ir/lower.rs`, `src/ir/verify.rs`, `src/ir/tests.rs`, `src/interp/machine.rs`, `src/interp/observation.rs` y `docs/validation/F0-0002-I04a-implementation-notes.md`. El API aislado `ir::compile_scalar` usa el AST y el checker propuestos; solo entrega un artefacto verificado a `interp::run_scalar`. La sustitución de tabla admite un donante verificado. `compile`, `nexo check` y `nexo run` siguen en HEAD.

Al retomar, ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`; exigir la línea final `0 differences` al comparador. Repetir R0103 con campos y span exactos, L04 por AST/resultado, el positivo escalar L07 en LF/CRLF/EOF, el oráculo decimal generado, las fronteras de observación y las pruebas de IR inválida. La medición de 15 familias se ejecuta de forma explícita. Conservar la laguna de offsets del corpus descrita en las notas y no atribuir a I04a arrays, recursos, llamadas Nexo, CFG, Result, efectos transitivos ni cuotas completas.

## Recuperación de I04b

I04b validada tras revisión independiente y publicada en `f5016d7`. Leer `docs/validation/F0-0002-I04b-implementation-notes.md` y revisar `src/ir/model.rs`, `src/ir/lower.rs`, `src/ir/verify.rs`, `src/ir/tests.rs` y `src/interp/machine.rs`. El API sigue siendo `ir::compile_scalar` y `interp::run_scalar`; estos nombres históricos incluyen ahora arrays de datos. `FunctionId` sigue siendo el nombre y el hash incluye raíces, índices, operadores y literales de array. El verificador impide ejecutar operaciones indexadas con tipos, raíces o permisos inválidos. Las lecturas anidadas comprueban cada límite antes de evaluar el índice siguiente; en contexto de lectura prestan el elemento. `drop(a)` libera el binding. Las escrituras calculan índices y RHS antes de comprobar límites. Los NX-E0105 independientes se ordenan por span.

Al retomar, ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`, confirmar `0 differences` y comparar con HEAD desde un artefacto independiente. Repetir el oráculo de 8.000 arrays, fuzz de 20.000, la sonda de pila 128/256/512 KiB y la medición explícita de quince familias. I05 quedó validada y publicada en `a203e51`; conservar sus límites nominales, los de I10 y la laguna `adr_gap_runtime_index_primary_tracks_whole_index_operation`. No atribuir a I04b recursos, llamadas Nexo, cuotas completas ni aprobación de F0.

## Recuperación de I05

I05 validada tras revisión independiente y publicada en `a203e51`. Base publicada: I01 `d4836cf`, I02a `f073275`, I02b `84d5f53`, I03 `f808a0c`, I04a `0739795`, I04b `f5016d7`. Revisar las notas `docs/validation/F0-0002-I05-implementation-notes.md` y los cambios de `src/ir/model.rs`, `lower.rs`, `verify.rs`, `tests.rs`, `i05_tests.rs`, `src/interp/machine.rs` y `observation.rs`. El API aislado conserva los nombres históricos `ir::compile_scalar` y `interp::run_scalar`.

Para recuperar el estado validado de I05, partir de `a203e51` y repetir `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`; el último debe terminar en `0 differences`. La evidencia de llamadas, recursos, asociación y orden escrito de argumentos, transferencia, retorno temprano, errores independientes y limpieza consta en `docs/validation/F0-0002-I05-implementation-notes.md`. `nexo check` y `nexo run` conservan la ruta publicada. I05 no acredita préstamos exclusivos, CFG, `Result`, efectos transitivos, cuotas completas ni aprobación de fase.

La corrección de I05 fija entidades `binding:<offset>` y `parameter:<offset>`, limpieza de temporales antes de locales en orden inverso y propagación de nombres envenenados tras `let` fallido. El lexer se comparte por archivo y la localización de parámetros usa su offset. El lowering del código inalcanzable restaura estado y consumo sin clon completo; la búsqueda del token de declaración conserva el span de un nombre sombreado. Las mediciones explícitas cubren 27 familias, incluidas 1.000 funciones, 48.000 parámetros, retornos anidados y declaraciones densas. La lectura indexada de una raíz consumida o destruida devuelve NX-E0202 con el sitio causal. I06 convierte el conflicto entre préstamo y transferencia de argumentos en NX-E1105 y acumula dos errores de propiedad independientes de una misma llamada. I10b conserva el span R0001 de unidad implícita `[9,len)` en vez de `}` y el tick ausente de `{}`. I05 fue validada y publicada en `a203e51`.

## Recuperación de I06

I06 validada tras revisión independiente y publicada en `0eaf876`. Base publicada: I01 `d4836cf`, I02a `f073275`, I02b `84d5f53`, I03 `f808a0c`, I04a `0739795`, I04b `f5016d7`, I05 `a203e51`; ADR de sintaxis `eca42dc`. Consultar `docs/validation/F0-0002-I06-implementation-notes.md` y revisar `src/ir/model.rs`, `lower.rs`, `verify.rs`, `i06_tests.rs` y `src/interp/machine.rs`. Los APIs aislados siguen siendo `ir::compile_scalar` e `interp::run_scalar`.

Al retomar, repetir las fixtures S06/S08/S30, P05–P07 con `data` y spans exactos (P07 `[73,77)` según §14.2), PP02, las lecturas posteriores a `inout`, los negativos de IR fabricada y el préstamo indexado sin celdas adicionales. Repetir también el modelo diferencial, las sondas de pila, la medición de doce familias y `sh scripts/compare-head.sh` con `0 differences`. El listado previo de I07a enumeraba 532 pruebas de biblioteca; el listado actual consta más abajo. El validador debe usar oráculos propios. `nexo check`/`run` permanecen en la ruta heredada; I07a está validada tras revisión independiente y publicada en `9f96780`; I07b está validada tras revisión independiente y publicada; I08–I11 y las puertas de F0 siguen pendientes. No declarar aprobada la fase por la evidencia local.


## Recuperación de I07a

El análisis sin IR ya se activa tras divergencia y en colas inalcanzables. Los repros de asignación e índice posteriores compilan; los de valor de rama ausente, propiedad y precedencia estructural devuelven los NX-E esperados. La escala con divergencia está medida en diez familias con controles vivos. El trabajo sigue incompleto hasta cubrir exhaustivamente las combinaciones de posición, rama, anotación y propiedad.

I07a validada tras revisión independiente y publicada en `9f96780`. Base publicada `0eaf876`; las notas están en `docs/validation/F0-0002-I07a-implementation-notes.md`. Revisar `src/check/types.rs`, `src/ir/model.rs`, `lower.rs`, `verify.rs`, `tests.rs`, `i07a_tests.rs`, `src/interp/machine.rs` y los contadores de escala. Repetir las filas S11, P08, B01–B06, B08–B09, F14, F25–F26 y los positivos L02/L05/L06 con oráculos independientes, más bucles anidados, limpieza, cuotas, IR malformada, fuzz y sondas de pila. La batería `ir::` terminó con 331 pruebas activas y 11 mediciones ignoradas; el listado de biblioteca enumera 580 pruebas en debug y 582 en release. Una medición explícita de diez familias, cada una con fuente divergente y control vivo, mantuvo pasos, bytes y asignaciones por debajo de razón 4,6 de 256 KiB a 1 MiB. Una copia cuadrática inyectada en el análisis muerto falló la medición reducida y el archivo se restauró con SHA-256 idéntico. Quedan pendientes la matriz combinatoria exhaustiva y la cardinalidad de reglas no muestreadas; el fuzz dirigido y la campaña de mutaciones del corte anterior constan en las notas. I07b debe resolver P09 y la propiedad que requiere punto fijo sobre retroaristas. La ruta heredada `nexo check`/`run` y las puertas de F0 siguen pendientes.

El corte anterior conserva regresiones para los nueve repros bloqueantes. Para reproducir el coste de retroaristas, ejecutar `cargo test --locked --offline --lib ir::i07a_tests::loop_backedges_scale_in_steps_allocation_and_time -- --ignored --nocapture`; tres familias entre 256 KiB y 1 MiB dieron razón máxima 4,335 en pasos y tráfico. El regresor activo detectó la reintroducción del barrido cuadrático y el mutador restauró el SHA-256. `sh scripts/validate.sh` terminó en 0 y `sh scripts/compare-head.sh` devolvió `0 differences`. Los listados contienen 580 pruebas de biblioteca en debug y 582 en release; 11 están ignoradas por perfil. El fuzz con oráculo propio de 100.000 fuentes, 42 mutaciones nuevas y las supervivientes nominales se ejecutaron sin supervivientes. La evidencia local queda INCOMPLETA hasta fijar los negativos aislados con campos exactos de las combinaciones restantes. I07a validada tras revisión independiente y publicada en `9f96780`.


Para reproducir el estado local, ejecutar `cargo test --locked --offline --lib ir::`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`.

El resultado de expresion distingue `Flow::Value { temp: Option<TempId>, ty }` de `Flow::Diverged { ty: Option<Ty> }`. Literales, nombres, indices, operadores, llamadas, arrays, `if` y bloques de valor usan ese resultado en el recorrido vivo y analizado. `emit` devuelve un temporal opcional. Quedan nueve apariciones de `usize::MAX`: siete son destinos CFG aun sin enlazar, una es el retorno ficticio de `finish_block` durante el analisis y una pertenece a un assert de test. Ninguna decide la divergencia de expresiones. Los usos de `== Ty::Never` comprueban el tipo interno inferior de bindings sin anotacion cuyo valor inicial diverge; no son centinelas de flujo. El tipo de los operadores procede de su operador y operandos presentes, con independencia del lado divergente. Un `()` derecho de suma o resta numerica da NX-E0102/type_mismatch con `expected_type=i64` y `found_type=()`. La decision `adr_gap_never_bottom` trata `Never` como fondo del reticulo: en igualdad y arrays no impone `i64`; el primer operando concreto fija el tipo y el diagnostico recae sobre el que lo contradice. La prioridad de NX-E1302/missing_branch_value se aplica a la funcion en el mismo recorrido vivo y analizado.

La regla vigilada de §13.9 conserva cada sitio independiente con izquierdo divergente: el error numérico del operando derecho se emite en el checker y se ordena por span junto con los demás errores de la función. La comprobación estructural de `value:` conserva prioridad cuando falta un valor de rama. El oráculo local comparó 300.000 pares de 67 plantillas, incluidas llamadas, arrays, binarios y hasta cuatro errores por función, con código, subcausa, etapa, operación, `data` y spans mapeados: cero diferencias en esa muestra, cero `Never` en `data` y ningún fallo interno. La muestra no acredita cobertura exhaustiva. La medición previa de 76 familias registró razones máximas de 4,14 en bytes y asignaciones; el nuevo recorrido de errores requiere una medición independiente de escala.

`cargo test --locked --offline --lib -- --list` enumera 607 pruebas en debug y 609 en release, con 12 ignoradas por perfil; las activas son 595 y 597. La batería `ir::` contiene 349 activas y 12 ignoradas; `check::` contiene 98 activas. Las veinte mutaciones nuevas de las reglas de operandos y `Never` fueron detectadas en debug y release con restauración SHA-256. I07a validada tras revisión independiente y publicada en `9f96780`. Siguen abiertas la matriz combinatoria exhaustiva, la revisión independiente y las puertas de fase del propietario. `sh scripts/validate.sh` terminó en 0 y `sh scripts/compare-head.sh` devolvió `0 differences`.

Los límites `adr_gap_deferred_var_alias` y `OutsideScalarSlice("lowering depth")` constan en las notas de I07a. I07a validada tras revisión independiente y publicada en `9f96780`.

## Recuperación de I07b

I07b validada tras revisión independiente y publicada. Revisar `src/ir/lower.rs`, `ownership_flow.rs`, `ownership_flow/sparse.rs` e `i07b_tests.rs` junto con `docs/validation/F0-0002-I07b-implementation-notes.md`. La condición completa del `while` entrega el mismo estado unido a cuerpo y salida falsa. El trie conserva los efectos diferidos incluso en hojas e hijos derechos. Una cola por índice deja converger las retroaristas antes de procesar la cadena posterior.

La cuota de propiedad se reinicia por función. NX-E0003 conserva solo errores de bloques estables y bucles convergidos; descarta observaciones intermedias y evita dos diagnósticos del mismo uso. Las funciones posteriores siguen analizándose. Repetir `cargo test --locked --offline --lib ir::`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`, con `0 differences`, y comprobar una copia LF. La comparación sin evaluación de valores, las cascadas y la cadena causal directa son límites nominales documentados. I07a está publicada en `9f96780`; el corpus independiente de terminaciones está en `45886fd` y CI en `f6f94b9`/`97257cd`. I08–I11 y las puertas de fase siguen pendientes.

El listado de biblioteca contiene 673 pruebas en debug y 675 en release, con 19 ignoradas por perfil; `ir::` ejecuta 408 activas en debug y 408 en release. `sh scripts/validate.sh` terminó en 0 en el árbol principal y en la copia LF; `sh scripts/compare-head.sh` devolvió `0 differences`.
