# Nexo — Pendientes

## Estado
- Base 0.2 recibida y preservada. Acuerdos posteriores: docs/F0_ACUERDOS.md.
- El propietario asigna implementación al asistente y validación a subagente separado.
- F0: EN EXPLORACIÓN. F1–F7: NO APROBADAS.
- Documentos respaldados en c4f020b; primer corte ejecutable con revisión independiente PASS limitada.

## Siguiente corte
1. Leer HANDOFF.md; comprobar respaldo remoto de F0-0001 y reproducir pruebas.
2. Continuar con I03 (nombres, asociación y tipos básicos) según F0-0002 §17.
3. Revisión independiente, regresiones y respaldo verificado en cada corte.

## Evidencia del primer corte
- Parser, checker de tipos/propiedad, intérprete de AST y CLI check/run.
- 22 tests de integración (19 semánticos/corpus y 3 CLI), debug y release.
- fmt y clippy estricto pasan. Corpus: 5 positivos y 10 negativos por código.
- La revisión detectó altura compuesta del AST y precedencia de límites negativos;
  y truncamiento UTF-8 al limitar la lectura, corregidos con regresiones. Ver docs/validation/F0-0001-review.md para veredicto.
- Recursos simulados, copia profunda física y límite explícito de trabajo.
- No hay IR bajada, préstamos, efectos, closures ni generadores ejecutables.

## Diseño F0-D11/D12/D13
- ADR D11 (closures), D12 (async), D13 (expresiones/módulos) y catálogo F0-CODIGOS: PASS de diseño en
  iteración 7 (docs/validation/F0-D11-D13-design-review.md). Sin ejecutar; NB1-NB8 abiertos.
- ADR F0-0002 (llamadas, IR, préstamos, efectos, Result/?): PASS de diseño en iteración 6 (docs/validation/F0-0002-design-review.md). Addendum de códigos pendiente de fusionar en F0-CODIGOS.md.
- Siguiente: Flujo B, sub-tareas I01-I11 de F0-0002 §17 (Codex Luna implementa; Sonnet valida).

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
- Estado por subtarea: I01 validada tras revisión independiente y publicada en `d4836cf`; I02a validada tras revisión independiente y publicada en `f073275`; I02b validada tras revisión independiente y publicada en main; I03–I11 no iniciadas.
- El addendum F0-CODIGOS-ADDENDUM-0002 sigue pendiente de fusionar en F0-CODIGOS.md.
- Diseño D11–D13 y F0-0002 con PASS de diseño; no equivale a aprobación del propietario.
- I01 abarca esquema, validación, JSON/texto, explain y lectura acotada; está validada tras revisión independiente y publicada en `d4836cf`. Las puertas de F0 siguen pendientes.
- Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.
- Límites conocidos de I01: las reglas de emisión y las inserciones vacías del mismo punto se comprueban en los emisores; los valores de cuota del perfil normal y la relación de `observed` como obligación del emisor constan como límites del esquema.

## Corte I02a
- Estado: léxico, tokens y spans validados tras revisión independiente y publicados en `f073275`.
- El lexer propuesto vive en `src/frontend/lex.rs`; el frontend de `syntax.rs` y la salida actual de `check`/`run` siguen separados.
- Incluye símbolos, identificadores/reservadas, enteros, decimal reducido, cadenas reconocidas, comentarios `//`, saltos LF/CR/CRLF y NX-E0001 léxico. LX01–LX05 tienen negativos y positivos pareados en la suite unitaria.
- I02b (parser, terminadores sintácticos, AST e impresor) está validada tras revisión independiente y publicada en main. L01–L07 y el resto de §14 no se cerraron en I02a.
- Ambigüedades de literales entre comillas y diagnóstico de lexema incompleto quedan registradas en `docs/validation/F0-0002-I02a-implementation-notes.md`; no se implementa decodificación semántica de cadenas.
- Siguiente paso: revisión independiente de I02b.

## Corte I02b

I02b está validada tras revisión independiente y publicada en main; I03–I11 no iniciadas. El frontend propuesto sigue separado de `compile`, `check` y `run`. Los límites estructurales de análisis y las regresiones de robustez se describen en `docs/validation/F0-0002-I02b-implementation-notes.md`.

Pendientes nominales: enumerar todas las posiciones recursivas de `PlaceRoot`, listas/mapas/restricciones de `WithClause` y alternativas inválidas de `Statement` y `Program` más allá de los 55 prefijos comprobados; completar negativos aislados para cada continuación y contexto de terminador, incluida no inserción estructural en LF/CRLF/CR/EOF. Conservar las lagunas `adr_gap_*` enumeradas en las notas de I02b. El fallo de creación del hilo ancho tiene camino diagnóstico sin panic; su alcance excepcional está documentado en las notas. Las obligaciones semánticas y de emisión de I03–I11 siguen abiertas. Las puertas generales de F0, en particular dos revisores sin discrepancias y aprobación del propietario, permanecen pendientes.
