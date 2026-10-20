# Nexo — Pendientes

## Estado
- Base 0.2 recibida y preservada. Acuerdos posteriores: docs/F0_ACUERDOS.md.
- El propietario asigna implementación al asistente y validación a subagente separado.
- F0: EN EXPLORACIÓN. F1–F7: NO APROBADAS.
- Documentos respaldados en c4f020b; primer corte ejecutable con revisión independiente PASS limitada.

## Siguiente corte
1. Leer HANDOFF.md; comprobar respaldo remoto de F0-0001 y reproducir pruebas.
2. Registrar D11–D13 con reglas y contraejemplos concretos sin alterar la base.
3. Extender prototipo por cortes: llamadas/IR/efectos y flujos reducidos de F0.
4. Revisión independiente, regresiones y respaldo verificado en cada corte.

## Evidencia del primer corte
- Parser, checker de tipos/propiedad, intérprete de AST y CLI check/run.
- 22 tests de integración (19 semánticos/corpus y 3 CLI), debug y release.
- fmt y clippy estricto pasan. Corpus: 5 positivos y 10 negativos por código.
- La revisión detectó altura compuesta del AST y precedencia de límites negativos;
  y truncamiento UTF-8 al limitar la lectura, corregidos con regresiones. Ver docs/validation/F0-0001-review.md para veredicto.
- Recursos simulados, copia profunda física y límite explícito de trabajo.
- No hay IR bajada, préstamos, efectos, closures ni generadores ejecutables.

## Decisiones y puertas aún pendientes
- D11 closures; D12 async; D13 expresiones/visibilidad/paquetes.
- Protocolo explícito de la propuesta H-PY 10×/3×; no resultado medido.
- D01–D10 completos, estados D09, coherencia de traits, gramática completa e IR/backend.
- Protocolos T01–T28/H-*/T26, plataformas y herramientas de referencia.
- Dos revisores sin discrepancias conforme a F0; un subagente no equivale a una
  auditoría externa ni elimina otros requisitos del roadmap.
- EX01/EX04 completos, generadores y biblioteca real corresponden a F2.

No ocultar pendientes al cambiar de workspace. No publicar secretos ni usar force push.
