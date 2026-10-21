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

I01 está validada tras revisión independiente y publicada en `d4836cf`; I02a está validada tras revisión independiente y publicada en `f073275`; I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada en `f5016d7`; I05 validada tras revisión independiente y publicada; I06–I11 no iniciadas. El addendum de códigos sigue pendiente de fusionar. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.

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

Al retomar, ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`, confirmar `0 differences` y comparar con HEAD desde un artefacto independiente. Repetir el oráculo de 8.000 arrays, fuzz de 20.000, la sonda de pila 128/256/512 KiB y la medición explícita de quince familias. Conservar los límites nominales de I05/I10 y la laguna `adr_gap_runtime_index_primary_tracks_whole_index_operation`. No atribuir a este corte recursos, llamadas Nexo, cuotas completas ni aprobación de F0.

## Recuperación de I05

I05 validada tras revisión independiente y publicada. Base publicada: I01 `d4836cf`, I02a `f073275`, I02b `84d5f53`, I03 `f808a0c`, I04a `0739795`, I04b `f5016d7`. Revisar las notas `docs/validation/F0-0002-I05-implementation-notes.md` y los cambios de `src/ir/model.rs`, `lower.rs`, `verify.rs`, `tests.rs`, `i05_tests.rs`, `src/interp/machine.rs` y `observation.rs`. El API aislado conserva los nombres históricos `ir::compile_scalar` y `interp::run_scalar`.

Antes de revisar, comprobar que el local no está detrás del remoto. Repetir `cargo test --locked --offline`, `sh scripts/validate.sh`, la medición explícita de I05 y `sh scripts/compare-head.sh`; el último debe terminar en `0 differences`. Contrastar con oráculos propios llamadas y recursos, asociación y orden escrito de argumentos, transferencia, retorno temprano, errores independientes, limpieza y los límites nominales. Comparar `nexo check` y `nexo run` contra HEAD con un artefacto separado. No atribuir a I05 préstamos exclusivos, CFG, `Result`, efectos transitivos, cuotas completas, aprobación de fase ni revisión independiente ya realizada.

La corrección local fija entidades `binding:<offset>` y `parameter:<offset>`, limpieza de temporales antes de locales en orden inverso y propagación de nombres envenenados tras `let` fallido. El lexer se comparte por archivo y la localización de parámetros usa su offset. El lowering del código inalcanzable restaura estado y consumo sin clon completo; la búsqueda del token de declaración conserva el span de un nombre sombreado. Las mediciones explícitas cubren 27 familias, incluidas 1.000 funciones, 48.000 parámetros, retornos anidados y declaraciones densas. La lectura indexada de una raíz consumida o destruida devuelve NX-E0202 con el sitio causal. Conservar como límites nominales la cardinalidad de dos errores independientes en un enunciado y el conflicto entre préstamo y transferencia de argumentos, que aún produce `InvalidIr("transfer while borrowed")` y corresponde a I06. I10b conserva el span R0001 de unidad implícita `[9,len)` en vez de `}` y el tick ausente de `{}`. La revisión independiente de la corrección sigue pendiente.
