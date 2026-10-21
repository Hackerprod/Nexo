# Nexo — Pendientes

## Estado
- Base 0.2 recibida y preservada. Acuerdos posteriores: docs/F0_ACUERDOS.md.
- El propietario asigna implementación al asistente y validación a subagente separado.
- F0: EN EXPLORACIÓN. F1–F7: NO APROBADAS.
- Documentos respaldados en c4f020b; primer corte ejecutable con revisión independiente PASS limitada.

## Siguiente corte
1. Leer HANDOFF.md; comprobar respaldo remoto de F0-0001 y reproducir pruebas.
2. Implementar I05 (frames, llamadas Nexo, recursos y limpieza) según F0-0002 §17; I04b ya fue validada y publicada.
3. Revisión independiente, regresiones y respaldo verificado en cada corte.

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
- ADR F0-0002 (llamadas, IR, préstamos, efectos, Result/?): PASS de diseño en iteración 6 (docs/validation/F0-0002-design-review.md). Addendum de códigos pendiente de fusionar en F0-CODIGOS.md.
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
- Estado por subtarea: I01 validada tras revisión independiente y publicada en `d4836cf`; I02a validada tras revisión independiente y publicada en `f073275`; I02b validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada; I05–I11 no iniciadas.
- El addendum F0-CODIGOS-ADDENDUM-0002 sigue pendiente de fusionar en F0-CODIGOS.md.
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
- I02b ya está publicada; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada.

## Corte I02b

I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada en `f808a0c`; I04a validada tras revisión independiente y publicada en `0739795`; I04b validada tras revisión independiente y publicada; I05–I11 no iniciadas. El frontend propuesto sigue separado de `compile`, `check` y `run`. Los límites estructurales de análisis y las regresiones de robustez se describen en `docs/validation/F0-0002-I02b-implementation-notes.md`.

Pendientes nominales: enumerar todas las posiciones recursivas de `PlaceRoot`, listas/mapas/restricciones de `WithClause` y alternativas inválidas de `Statement` y `Program` más allá de los 55 prefijos comprobados; completar negativos aislados para cada continuación y contexto de terminador, incluida no inserción estructural en LF/CRLF/CR/EOF. Conservar las lagunas `adr_gap_*` enumeradas en las notas de I02b. El fallo de creación del hilo ancho tiene camino diagnóstico sin panic; su alcance excepcional está documentado en las notas. Las obligaciones semánticas y de emisión posteriores a I03 siguen abiertas. Las puertas generales de F0, en particular dos revisores sin discrepancias y aprobación del propietario, permanecen pendientes.

## Corte I03

I03 validada tras revisión independiente y publicada. El módulo aislado `check::check_proposed` cubre registro de firmas, duplicados, ámbitos, sombreado, asociación estructural, modos estructurales, entrada básica, literales y tipos independientes; cierra los negativos L01 y L07 en las etapas que asigna §17.2; sus positivos completos siguen asignados a pasos posteriores. La ruta heredada de `check`/`run` conserva su comportamiento. Consultar `docs/validation/F0-0002-I03-implementation-notes.md` para evidencia y límites.

Pendientes nominales: cierre completo de los positivos y observaciones de S01–S34 según §17.2; ejecución y propiedad de argumentos en I05–I06; análisis de caminos y valores de ramas en I07; semántica completa de Result, `?`, inferencia de constructores y puerta completa de entrada en I08; efectos en I09; cuotas completas en I10; integración en I11. Permanecen la fusión del addendum de códigos, dos revisores sin discrepancias y la aprobación del propietario para F0.

La duración actual de la suite completa y las razones de escalado de 256 KiB a 1 MiB constan en las notas de I03; la prueba preexistente de escalado del parser domina la duración. Límites conocidos: detección sintáctica de `resource_array` por nombre de intrínseco, guardas defensivas de profundidad con contadores nominales, inferencia incompleta de `Ok(Ok(1))`, mensaje semántico genérico, contexto de tipo esperado en arrays heterogéneos y tipo diferido de `?`. Las cuatro interpretaciones locales de cardinalidad de §13.9 tienen pruebas `adr_gap_*` y se describen en las notas.

## Corte I04a

I04a validada tras revisión independiente y publicada. IR escalar verificada, identidades y hash de funciones, tabla sustituible en desarrollo, bindings, reasignación completa, aritmética comprobada, comparaciones, `print`, `assert_eq`, `drop` de datos, `fail` y registro ordenado de salida están aislados de los comandos actuales. Las pruebas locales de IR y ejecución se enumeran en `docs/validation/F0-0002-I04a-implementation-notes.md`; `cargo test --locked --offline --lib -- --list` enumera 257 pruebas, incluida una medición de escalado de lowering de ejecución explícita. Los resultados locales no son revisión independiente.

Pendientes nominales: I05 debe introducir frames, llamadas de usuario, recursos, retorno y limpieza secuencial. I06–I09 conservan préstamos, CFG, Result y efectos; I10 debe cerrar cuotas y contabilidad completa; I11 integra CLI y flujos. Los positivos de §14 que requieren esas capacidades no se atribuyen a I04a. Permanecen los requisitos generales de F0, incluidos dos revisores sin discrepancias y aprobación del propietario.

## Corte I04b

I04b validada tras revisión independiente y publicada. La IR verificada y el intérprete aislado cubren arrays de datos, copias profundas, lectura y escritura indexadas, `len`, igualdad estructural y observación; las cuotas R0002 de celdas incluyen cabeceras y elementos. El negativo `immutable_write.nx` emite NX-E0105 en ownership con raíz y declaración relacionadas; los sitios independientes se acumulan en orden de fuente. `nested_copy` ejecuta copias sin alias. `drop(a)` libera el binding antes de crear la unidad; las lecturas indexadas prestan el elemento y las asignaciones o declaraciones propietarias copian. `docs/validation/F0-0002-I04b-implementation-notes.md` enumera las filas de §17.2, evidencia local y lagunas. La suite enumera 311 pruebas de biblioteca; la revisión separada y las puertas de F0 siguen pendientes.

Límites nominales conservados: ticks de `work_consumed` frente a §11.6 e `ir_items` corresponden a I10; el rechazo NX-E0202 del uso tras `drop` corresponde a I05. El span de R0002 de la unidad final implícita cubre el cuerpo, no la `}`; el ADR no fija span de R0102 ni `expected_type` relacionado para asignación indexada. El código posterior a `return` obtiene `outside` en lugar de NX-E0105, heredado de I04a. I05–I11 conservan recursos, llamadas Nexo, permisos generales, CFG, Result, efectos, cuotas completas e integración. El addendum de códigos y la puerta de dos revisores sin discrepancias siguen abiertos.
