# Revisión de la implementación F0-0002 / I01 (Flujo B)

Alcance: sub-tarea I01 de docs/adr/F0-0002-second-slice.md (§17): esquema diagnóstico, spans,
presentación, catálogo de `explain` y límites de entrada. Implementación: Codex (gpt-6-luna).
Validador: Sonnet 5.5, instancia nueva y sin contexto en cada ronda. Coordinación: Claude (juez).

**Veredicto del corte: PASS en la iteración 7 (validación acotada final).** Vale para I01, no para
F0 ni F0-0002 completo; no sustituye "dos revisores sin discrepancias" ni la aprobación del
propietario. I02–I11 no están iniciadas.

## Historial

| Iter | Veredicto | Defectos bloqueantes principales |
|---|---|---|
| 1 | FAIL | Subcausas de NX-E0112 ausentes y catálogo parseado del Markdown; disponibilidad incorrecta; `explain` con fuentes vacías; panic con argumento no Unicode; `invalid_utf8`/`source_bytes` no serializables; validación por código incompleta; tests que no detectaban mutaciones |
| 2 | FAIL | REGRESIÓN en `check`/`run` con UTF-8 truncado al final; claves de Entity sin orden en texto; validación por código incompleta; `invalid_utf8` no renderizable en texto; mutaciones sensibles sin detectar; afirmación falsa en las notas |
| 3 | FAIL | Contrato léxico/sintáctico (§13.14) sin validar; tabla etapa/operación de los R; contrato de entrada y datos; 25 mutaciones supervivientes; afirmación inexacta sobre Git; documentos con referencias de proceso |
| 4 | FAIL | Falsos positivos (perfiles reducidos de §11.4; NX-E0005 sin expresión); R0001 sin `observed = limit+1`; NX-E0003 sin `primary_span`; 5 mutaciones; pérdida de contenido en documentos de estado |
| 5 | FAIL | Perfiles reducidos sin heredar cuotas; correspondencia Cause.kind↔operation no total; hueco de test en NX-E1105 |
| 6 | FAIL | Paso 1 de Pending.md abreviado; test de span vacío que no aislaba la regla |
| 7 | PASS | Ninguno |

## Decisiones del juez

- `explain` devuelve UN ejemplo (representante) por código+subcausa, comparado literalmente con §14.
- Lagunas reales del ADR (subcausas sin fixture publicada, NX-E0004, NX-E0203, consecuencias de
  reparación no publicadas): campos vacíos y registro, sin inventar contenido.
- Obligaciones del emisor (reglas de emisión de §13.15, valores de cuota del perfil normal y
  relaciones de `observed` en los R): no se validan en el esquema; límite conocido.
- Presupuestos de análisis costosos: span primario no comparado por igualdad exacta (solo
  existencia y no vaciedad, §13.13).

## No bloqueantes abiertos

- a8c: mutante equivalente (falta de `limit` rechazada por la regla del límite, distinto mensaje).
- El mensaje del span vacío en kinds estructurales dice "costly analysis spans are nonempty".
- Dos viñetas casi redundantes en Pending.md.
- diagnostic.rs ronda las 3.900 líneas (§10 asigna un único módulo `diagnostic`).
- `explain` repite la descripción completa del código en cada subcausa.

## Límites

Validación por lectura, ejecución de la suite y mutaciones en copias; sin red. Pendiente: emisores
reales (I02–I11), ejecución de las fixtures de §14 por el compilador, fusión del addendum de
códigos en F0-CODIGOS.md.
