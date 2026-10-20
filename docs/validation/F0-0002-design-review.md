# Revisión de diseño F0-0002 (Flujo A)

Alcance: docs/adr/F0-0002-second-slice.md y docs/adr/F0-CODIGOS-ADDENDUM-0002.md (inserciones
propuestas al catálogo F0-CODIGOS.md, aún NO fusionadas). Diseño: Codex (gpt-6-astra).
Validador: Sonnet 5.5, instancia nueva y sin contexto en cada ronda. Coordinación: Claude (juez).

**Veredicto del corte: PASS en la iteración 6.** Vale para este corte de diseño. No aprueba F0 ni
F1, no ejecuta nada (ni compilador ni fixtures) y no sustituye "dos revisores sin discrepancias" ni
la aprobación del propietario. Todas las filas y totales son expectativas de diseño.

## Historial

| Iter | Veredicto | Bloqueantes principales |
|---|---|---|
| 1 | FAIL | Cuota live_value_cells inalcanzable; fila B03 ambigua; interfaz observable sin definir; códigos y migración del corpus sin cerrar |
| 2 | FAIL | Esquema JSON de diagnostics sin definir; contabilidad de trabajo ambigua |
| 3 | FAIL | Orden de sub-tareas con dependencias posteriores; ejemplo NX-E0005 contradice D13; valores comparables sin determinar (stage, primary_span, rellenos E0002, vocabularios) |
| 4 | FAIL | Span de NX-E0003 no determinista en 4 presupuestos de análisis; ejemplo JSON parcheado con erratum; `data` de NX-E1105 sin determinar |
| 5 | FAIL | I04a dependía de construcciones de I04b |
| 6 | PASS | Ninguno |

## Decisiones del juez

- Para `type_solver_steps`, `effect_fixpoint_steps`, `ir_items` y `ownership_flow_steps` el modelo
  independiente NO compara el primary_span (solo que exista, esté dentro del archivo y no sea vacío);
  se comparan budget_kind, limit, observed, código y etapa (§13.13).

## No bloqueantes abiertos (iter 6)

1. K01/K02 migrados sin paso asignado en §17 (caben en I02).
2. Corpus migrado y programas de §14.5 sin paso explícito (caen en I05 o posterior).
3. Nota de §19 no registra el FAIL de la ronda 5 ni la corrección I04a→I04b.
4. L07 podría ir en I04a. 5. P04 se cierra en I05 aunque el catálogo de E1105 crece en I06.
6. Formato: líneas en blanco antes de ## 15 y ## 18; kinds de token con mayúscula/minúscula.

## Limites

Verificación por lectura y scripts de solo lectura: ticks 56/25/15 y 999.992+p, 4 familias de
análisis, ~40 spans en LF y CRLF, 66+4=70 códigos, offsets de EX01/EX04, migración de 15 archivos
y 22 tests. Pendiente: implementar (Flujo B), ejecutar fixtures y arneses, fusionar el addendum
en F0-CODIGOS.md y contrastar con parser/formateador reales.
