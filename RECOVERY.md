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

I01 está validada tras revisión independiente y publicada en `d4836cf`; I02a está validada tras revisión independiente y publicada en `f073275`; I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada; I04–I11 no iniciadas. El addendum de códigos sigue pendiente de fusionar. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.

## Recuperación de I02a

I02a está validada tras revisión independiente y publicada en `f073275`. El lexer nuevo está aislado en `src/frontend/lex.rs` y se exporta bajo `frontend::lex`; no se ha conectado a `compile`, `check` o `run`. Consultar `docs/validation/F0-0002-I02a-implementation-notes.md` para alcance, lagunas y suites. I02b está validada tras revisión independiente y publicada en `84d5f53`; el frontend propuesto sigue aislado de la CLI hasta el paso de integración asignado por el ADR.

## Recuperación de I02b

I02b está validada tras revisión independiente y publicada en `84d5f53`. `frontend::parse` y `frontend::print` siguen aislados de `check`/`run`; el estado técnico, las lagunas del ADR, las pruebas y los límites conocidos están en `docs/validation/F0-0002-I02b-implementation-notes.md`. Antes de continuar, comprobar `git status`, comparar con el remoto según el flujo de agentes y ejecutar `cargo test --locked --offline`, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`. La aprobación de fase F0 sigue reservada al propietario.

La terminación se decide al descender por cada sentencia y los diagnósticos FIRST usan el estado de la producción comprometida. El inicio de cada sentencia se guarda durante el descenso; los operadores excluidos conservan su prefijo exacto tras LF, CRLF, CR y comentarios. El parser usa un hilo ancho reutilizable incluso para anidamientos bajos, con diagnóstico si falla su creación. La suite de regresión incluye prefijos repetidos, `?`, sombreado, ramas de retorno, fronteras de presupuestos, lista de operaciones vacía, inserción tras trivia y pilas pequeñas mediante subprocesos. Revisar los límites nominales de posiciones FIRST y terminadores antes de cerrar I02b.

## Recuperación de I03

I03 está validada tras revisión independiente y publicada. Revisar `src/check/names.rs`, `src/check/types.rs` y `src/check/tests.rs` junto con `docs/validation/F0-0002-I03-implementation-notes.md`. El API `check::check_proposed` es aislado y devuelve una lista ordenada de diagnósticos semánticos; `compile`, `nexo check` y `nexo run` siguen usando la ruta anterior. Repetir los tests locked/offline, `sh scripts/validate.sh` y `sh scripts/compare-head.sh`. La salida del comparador debe terminar en `0 differences`. Registrar cualquier fallo con una regresión antes de ampliar el alcance; no atribuir a I03 ejecución de llamadas, préstamos, efectos, control por caminos ni Result completo. I03 pasó revisión independiente en su tercera ronda.

Al retomar I03, repetir primero la prioridad entre nombres y argumentos, los spans de anotación acabada en `>`, el salto que terminó `return`, la cardinalidad de errores independientes, las ramas que retornan, el tipo diferido de `?`, la sonda de pilas pequeñas y la restauración de ámbitos y contexto de tipos. La prueba de escalado usa 16 familias por defecto y 24 con `NEXO_FULL_SCALE`, de 256 KiB a 1 MiB. La duración actual está en las notas de I03; la prueba de escalado preexistente del parser domina la suite.
